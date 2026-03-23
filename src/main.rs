mod models;
mod graph;
mod protocol;
mod registry;
mod storage;
mod session;
mod dispatcher;

use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, HeaderMap},
    response::{sse::{Event, Sse}, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use models::MemoryScope;
use protocol::*;
use registry::GraphRegistry;
use serde::Deserialize;
use serde_json::{json, Value};
use session::SessionManager;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_stream::wrappers::ReceiverStream;
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn, debug};
use tracing_subscriber::EnvFilter;
use dispatcher::protocol_handle_request;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ServerArgs {
    /// Root directory for all memory storage
    #[arg(short, long, env = "MCP_STORAGE_ROOT", default_value = "storage")]
    root: String,

    /// Directory for guideline documents (effective_work.md, etc.)
    #[arg(long, env = "MCP_DOCS_DIR", default_value = "docs")]
    docs_dir: String,

    /// API Key for Bearer authentication (optional)
    #[arg(long, env = "MCP_API_KEY")]
    api_key: Option<String>,

    /// HTTP server host
    #[arg(long, env = "MCP_HTTP_HOST", default_value = "127.0.0.1")]
    host: String,

    /// HTTP server port
    #[arg(short, long, env = "MCP_HTTP_PORT", default_value_t = 3000)]
    port: u16,

    /// Execution mode
    #[arg(short, long, value_enum, default_value_t = Mode::Hybrid)]
    mode: Mode,

    /// Project ID for stdio mode
    #[arg(long, env = "MCP_PROJECT_ID", default_value = "default")]
    project_id: String,

    /// Agent ID for stdio mode (if none, uses shared scope)
    #[arg(long, env = "MCP_AGENT_ID")]
    agent_id: Option<String>,

    /// Save interval in seconds
    #[arg(short, long, default_value_t = 30)]
    interval: u64,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum Mode {
    Stdio,
    Http,
    Hybrid,
}

struct AppState {
    registry: Arc<GraphRegistry>,
    sessions: Arc<SessionManager>,
    storage_root: String,
    docs_dir: String,
    api_key: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let args = ServerArgs::parse();
    info!("Starting MCP Memory Server v{} in {:?} mode", env!("CARGO_PKG_VERSION"), args.mode);

    let registry = Arc::new(GraphRegistry::new(&args.root));
    let sessions = Arc::new(SessionManager::new());
    let storage_root = args.root.clone();
    let docs_dir = args.docs_dir.clone();
    let api_key = args.api_key.clone();

    if api_key.is_some() {
        info!("API Key authentication enabled");
    } else if args.host != "127.0.0.1" && args.host != "localhost" {
        warn!("Server is bound to {} without API Key! This is insecure.", args.host);
    }

    // Background saver
    let saver_registry = Arc::clone(&registry);
    let interval = args.interval;
    let saver_handle = tokio::spawn(async move {
        let mut interval_timer = tokio::time::interval(Duration::from_secs(interval));
        loop {
            interval_timer.tick().await;
            saver_registry.save_all();
        }
    });

    // Background session cleanup
    let cleanup_sessions = Arc::clone(&sessions);
    let cleanup_handle = tokio::spawn(async move {
        let mut interval_timer = tokio::time::interval(Duration::from_secs(300)); // Every 5 mins
        loop {
            interval_timer.tick().await;
            let cleaned = cleanup_sessions.cleanup_inactive(Duration::from_secs(1800)); // 30 mins idle
            if cleaned > 0 {
                info!("Cleaned up {} inactive sessions", cleaned);
            }
        }
    });

    let mut http_handle = None;
    if args.port > 0 {
        let app_state = Arc::new(AppState {
            registry: Arc::clone(&registry),
            sessions: Arc::clone(&sessions),
            storage_root: storage_root.clone(),
            docs_dir: docs_dir.clone(),
            api_key: api_key.clone(),
        });

        // Hardened CORS
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderName::from_static("mcp-session-id"),
                axum::http::HeaderName::from_static("mcp-protocol-version"),
            ])
            .expose_headers(Any);

        let app = Router::new()
            .route("/sse/projects/{pid}/shared", get(handle_legacy_sse_shared))
            .route("/sse/projects/{pid}/agents/{aid}", get(handle_legacy_sse_agent))
            .route("/message", post(handle_legacy_message_post))
            .route("/mcp/projects/{pid}/shared", get(handle_streamable_get_shared).post(handle_streamable_post_shared))
            .route("/mcp/projects/{pid}/agents/{aid}", get(handle_streamable_get_agent).post(handle_streamable_post_agent))
            .layer(axum::middleware::from_fn_with_state(Arc::clone(&app_state), auth_middleware))
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(app_state);

        let listener = tokio::net::TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;
        info!("HTTP server listening on {}", listener.local_addr()?);
        
        http_handle = Some(tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!("HTTP server error: {}", e);
            }
        }));
    }

    if args.mode == Mode::Stdio || args.mode == Mode::Hybrid {
        let project_id = args.project_id.clone();
        let scope = match args.agent_id {
            Some(id) => MemoryScope::Agent(id),
            None => MemoryScope::Shared,
        };

        let mut reader = BufReader::new(io::stdin());
        let mut stdout = io::stdout();

        info!("Stdio mode active (Project: {}, Scope: {})", project_id, scope);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    if args.mode == Mode::Hybrid {
                        // In hybrid mode, if stdin is closed (e.g. background service), 
                        // we stay alive for HTTP requests.
                        debug!("Stdio reach EOF, but keeping HTTP server alive in Hybrid mode");
                        tokio::select! {
                            _ = tokio::signal::ctrl_c() => info!("Shutdown signal received"),
                            _ = async {
                                if let Some(h) = http_handle {
                                    let _ = h.await;
                                }
                            } => warn!("HTTP server exited unexpectedly"),
                        }
                        break;
                    } else {
                        break;
                    }
                }, 
                Ok(_) => {
                    let payload: RpcPayload = match serde_json::from_str(&line) {
                        Ok(req) => req,
                        Err(_) => continue,
                    };

                    let graph = registry.get_or_load(&project_id, scope.clone());
                    debug!(project_id, scope = %scope, "Handling stdio request");
                    let response_val = process_payload(&graph, payload, &project_id, &scope, None, &storage_root, &docs_dir).await;
                    
                    if let Some(res) = response_val {
                        let response_json = serde_json::to_string(&res).unwrap_or_default();
                        if let Err(e) = stdout.write_all(response_json.as_bytes()).await {
                            error!("Failed to write to stdout: {}", e);
                            break;
                        }
                        if let Err(e) = stdout.write_all(b"\n").await {
                            error!("Failed to write to stdout: {}", e);
                            break;
                        }
                        let _ = stdout.flush().await;
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
    } else {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => info!("Shutdown signal received"),
            _ = async {
                if let Some(h) = http_handle {
                    let _ = h.await;
                }
            } => warn!("HTTP server exited unexpectedly"),
        }
    }

    info!("Shutting down... Performing final save.");
    saver_handle.abort();
    cleanup_handle.abort();
    registry.save_all();
    Ok(())
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(ref expected_key) = state.api_key {
        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let authorized = if let Some(auth_str) = auth_header {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                token == expected_key
            } else {
                false
            }
        } else {
            false
        };

        if !authorized {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(request).await)
}

// ==================== Streamable HTTP (2025-11-25) ====================

async fn handle_streamable_get_shared(
    headers: HeaderMap,
    Path(pid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    create_streamable_session(headers, pid, MemoryScope::Shared, state).await
}

async fn handle_streamable_get_agent(
    headers: HeaderMap,
    Path((pid, aid)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    create_streamable_session(headers, pid, MemoryScope::Agent(aid), state).await
}

async fn create_streamable_session(
    headers: HeaderMap,
    project_id: String,
    scope: MemoryScope,
    state: Arc<AppState>,
) -> impl IntoResponse {
    let (session_id, rx) = state.sessions.create_session(project_id.clone(), scope.clone());
    info!("New Streamable session requested: {} for project: {}, scope: {}", session_id, project_id, scope);

    let mut headers_resp = HeaderMap::new();
    headers_resp.insert("mcp-session-id", session_id.parse().unwrap());
    headers_resp.insert("mcp-protocol-version", "2025-11-25".parse().unwrap());

    // Send initial endpoint event to help client discovery (some clients need this even in 2025 version)
    if let Some(session) = state.sessions.get_session(&session_id) {
        let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or("127.0.0.1:3000");
        let base_path = match scope {
            MemoryScope::Shared => format!("mcp/projects/{}/shared", project_id),
            MemoryScope::Agent(ref aid) => format!("mcp/projects/{}/agents/{}", project_id, aid),
        };
        let endpoint_uri = format!("http://{}/{}?session_id={}", host, base_path, session_id);
        let event = Event::default().event("endpoint").data(endpoint_uri);
        let _ = session.sender.send(Ok(event)).await;
    }

    let sse = Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::default());
    (headers_resp, sse)
}

#[derive(Deserialize)]
struct PostQuery {
    session_id: Option<String>,
}

// Helper for Streamable POST handlers
async fn handle_streamable_post_logic(
    project_id: String,
    scope: MemoryScope,
    Query(query): Query<PostQuery>,
    headers: HeaderMap,
    state: Arc<AppState>,
    Json(payload): Json<RpcPayload>,
) -> impl IntoResponse {
    let session_id_opt = headers.get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or(query.session_id);
    
    info!("Streamable POST request received. Session: {:?}, Project: {}, Scope: {}", session_id_opt, project_id, scope);
    
    let (pid_resolved, scope_resolved, version_ref): (String, MemoryScope, Option<Arc<RwLock<String>>>) = if let Some(ref sid) = session_id_opt {
        if let Some(session) = state.sessions.get_session(sid) {
            (session.project_id.clone(), session.scope.clone(), Some(session.protocol_version))
        } else {
            warn!("Session {} not found, using path params", sid);
            (project_id, scope, None) 
        }
    } else {
        (project_id, scope, None)
    };

    let graph = state.registry.get_or_load(&pid_resolved, scope_resolved.clone());
    let response_val = process_payload(&graph, payload, &pid_resolved, &scope_resolved, version_ref.clone(), &state.storage_root, &state.docs_dir).await;
    
    if let Some(sid) = session_id_opt {
        if let Some(session) = state.sessions.get_session(&sid) {
            if let Some(ref res) = response_val {
                let event = Event::default().data(serde_json::to_string(res).unwrap_or_default());
                let _ = session.sender.send(Ok::<Event, axum::Error>(event)).await;
            }
        }
    }

    let version = if let Some(ref vr) = version_ref {
        vr.read().map(|r| r.clone()).unwrap_or_else(|_| "2025-11-25".to_string())
    } else {
        "2025-11-25".to_string()
    };

    let mut headers = HeaderMap::new();
    headers.insert("mcp-protocol-version", version.parse().unwrap());

    match response_val {
        Some(res) => {
            headers.insert(axum::http::header::CONTENT_TYPE, "application/json; charset=utf-8".parse().unwrap());
            (StatusCode::OK, headers, Json(res)).into_response()
        },
        None => {
            (StatusCode::ACCEPTED, headers, "").into_response()
        },
    }
}

async fn handle_streamable_post_shared(
    Path(pid): Path<String>,
    Query(query): Query<PostQuery>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RpcPayload>,
) -> impl IntoResponse {
    handle_streamable_post_logic(pid, MemoryScope::Shared, Query(query), headers, state, Json(payload)).await
}

async fn handle_streamable_post_agent(
    Path((pid, aid)): Path<(String, String)>,
    Query(query): Query<PostQuery>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RpcPayload>,
) -> impl IntoResponse {
    handle_streamable_post_logic(pid, MemoryScope::Agent(aid), Query(query), headers, state, Json(payload)).await
}


// ==================== Legacy HTTP+SSE (2024-11-05) ====================

async fn handle_legacy_sse_shared(
    headers: HeaderMap,
    Path(pid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    create_legacy_sse_session(headers, pid, MemoryScope::Shared, state).await
}

async fn handle_legacy_sse_agent(
    headers: HeaderMap,
    Path((pid, aid)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    create_legacy_sse_session(headers, pid, MemoryScope::Agent(aid), state).await
}

async fn create_legacy_sse_session(
    headers: HeaderMap,
    project_id: String,
    scope: MemoryScope,
    state: Arc<AppState>,
) -> impl IntoResponse {
    let (session_id, rx) = state.sessions.create_session(project_id.clone(), scope.clone());
    debug!("New Legacy SSE session created: {} for {} / {}", session_id, project_id, scope);

    let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or("localhost:3000");
    let endpoint_uri = format!("http://{}/message?session_id={}", host, session_id);

    if let Some(session) = state.sessions.get_session(&session_id) {
        let event = Event::default()
            .event("endpoint")
            .data(endpoint_uri);
        let _ = session.sender.send(Ok(event)).await;
    }

    Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::default())
}

#[derive(Deserialize)]
struct MessageQuery {
    session_id: String,
}

async fn handle_legacy_message_post(
    headers: HeaderMap,
    Query(query): Query<MessageQuery>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RpcPayload>,
) -> impl IntoResponse {
    debug!("Legacy POST headers: {:?}", headers);
    debug!("Legacy POST message received for session: {}", query.session_id);
    
    let session_id = &query.session_id;
    let session_opt = state.sessions.get_session(session_id);
    
    let (project_id, scope, version_ref): (String, MemoryScope, Option<Arc<RwLock<String>>>) = if let Some(ref s) = session_opt {
        (s.project_id.clone(), s.scope.clone(), Some(s.protocol_version.clone()))
    } else {
        ("default".to_string(), MemoryScope::Shared, None)
    };

    let graph = state.registry.get_or_load(&project_id, scope.clone());
    let response_val = process_payload(&graph, payload, &project_id, &scope, version_ref.clone(), &state.storage_root, &state.docs_dir).await;
    
    if let (Some(session), Some(res)) = (session_opt, response_val.clone()) {
        let event = Event::default().data(serde_json::to_string(&res).unwrap_or_default());
        let _ = session.sender.send(Ok::<Event, axum::Error>(event)).await;
    }

    let mut headers = HeaderMap::new();
    match response_val {
        Some(res) => {
            headers.insert(axum::http::header::CONTENT_TYPE, "application/json; charset=utf-8".parse().unwrap());
            (StatusCode::OK, headers, Json(res)).into_response()
        },
        None => (StatusCode::ACCEPTED, headers, "").into_response(),
    }
}

// ==================== Payload Processor ====================

async fn process_payload(
    graph: &Arc<crate::graph::MemoryGraph>,
    payload: RpcPayload,
    project_id: &str,
    scope: &MemoryScope,
    session_version: Option<Arc<RwLock<String>>>,
    storage_root: &str,
    docs_dir: &str,
) -> Option<Value> {
    match payload {
        RpcPayload::Single(req) => {
            debug!(project_id = %project_id, scope = %scope, method = %req.method, "Handling RPC request");
            let response = protocol_handle_request(graph, req, session_version, storage_root, docs_dir).await;
            response.map(|r| serde_json::to_value(&r).unwrap_or(json!({})))
        }
        RpcPayload::Batch(reqs) => {
            debug!(project_id = %project_id, scope = %scope, batch_size = reqs.len(), "Handling RPC batch request");
            let mut responses = Vec::with_capacity(reqs.len());
            for req in reqs {
                if let Some(res) = protocol_handle_request(graph, req, session_version.clone(), storage_root, docs_dir).await {
                    responses.push(res);
                }
            }
            if responses.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&responses).unwrap_or(json!([])))
            }
        }
    }
}
