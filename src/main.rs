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

    let mut http_handle = None;
    if args.port > 0 {
        let app_state = Arc::new(AppState {
            registry: Arc::clone(&registry),
            sessions: Arc::clone(&sessions),
        });

        // Hardened CORS
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
            .expose_headers(Any);

        let app = Router::new()
            .route("/sse/projects/{pid}/shared", get(handle_legacy_sse_shared))
            .route("/sse/projects/{pid}/agents/{aid}", get(handle_legacy_sse_agent))
            .route("/message", post(handle_legacy_message_post))
            .route("/mcp/projects/{pid}/shared", get(handle_streamable_get_shared).post(handle_streamable_post_shared))
            .route("/mcp/projects/{pid}/agents/{aid}", get(handle_streamable_get_agent).post(handle_streamable_post_agent))
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(app_state);

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", args.port)).await?;
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
                Ok(0) => break, 
                Ok(_) => {
                    let payload: RpcPayload = match serde_json::from_str(&line) {
                        Ok(req) => req,
                        Err(_) => continue,
                    };

                    let graph = registry.get_or_load(&project_id, scope.clone());
                    debug!(project_id, scope = %scope, "Handling stdio request");
                    let response_val = process_payload(&graph, payload, &project_id, &scope, None).await;
                    
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
    registry.save_all();
    Ok(())
}

// ==================== Streamable HTTP (2025-11-25) ====================

async fn handle_streamable_get_shared(
    Path(pid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    create_streamable_session(pid, MemoryScope::Shared, state).await
}

async fn handle_streamable_get_agent(
    Path((pid, aid)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    create_streamable_session(pid, MemoryScope::Agent(aid), state).await
}

async fn create_streamable_session(
    project_id: String,
    scope: MemoryScope,
    state: Arc<AppState>,
) -> impl IntoResponse {
    let (session_id, rx) = state.sessions.create_session(project_id.clone(), scope.clone());
    debug!("New Streamable session created: {} for {} / {}", session_id, project_id, scope);

    let mut headers = HeaderMap::new();
    headers.insert("Mcp-Session-Id", session_id.parse().unwrap());
    headers.insert("Mcp-Protocol-Version", "2025-11-25".parse().unwrap());

    let sse = Sse::new(ReceiverStream::new(rx))
        .keep_alive(axum::response::sse::KeepAlive::default());
    (headers, sse)
}

// Helper for Streamable POST handlers
async fn handle_streamable_post_logic(
    project_id: String,
    scope: MemoryScope,
    headers: HeaderMap,
    state: Arc<AppState>,
    payload: RpcPayload,
) -> impl IntoResponse {
    debug!("Streamable POST headers: {:?}", headers);
    
    let session_id_opt = headers.get("mcp-session-id").and_then(|v| v.to_str().ok());
    
    let (pid_resolved, scope_resolved, version_ref): (String, MemoryScope, Option<Arc<RwLock<String>>>) = if let Some(sid) = session_id_opt {
        if let Some(session) = state.sessions.get_session(sid) {
            (session.project_id.clone(), session.scope.clone(), Some(session.protocol_version))
        } else {
            (project_id, scope, None) 
        }
    } else {
        (project_id, scope, None)
    };

    let graph = state.registry.get_or_load(&pid_resolved, scope_resolved.clone());
    let response_val = process_payload(&graph, payload, &pid_resolved, &scope_resolved, version_ref.clone()).await;
    
    if let Some(sid) = session_id_opt {
        if let Some(session) = state.sessions.get_session(sid) {
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
    headers.insert("Mcp-Protocol-Version", version.parse().unwrap());

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
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RpcPayload>,
) -> impl IntoResponse {
    handle_streamable_post_logic(pid, MemoryScope::Shared, headers, state, payload).await
}

async fn handle_streamable_post_agent(
    Path((pid, aid)): Path<(String, String)>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RpcPayload>,
) -> impl IntoResponse {
    handle_streamable_post_logic(pid, MemoryScope::Agent(aid), headers, state, payload).await
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
    let response_val = process_payload(&graph, payload, &project_id, &scope, version_ref.clone()).await;
    
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
) -> Option<Value> {
    match payload {
        RpcPayload::Single(req) => {
            debug!(project_id = %project_id, scope = %scope, method = %req.method, "Handling RPC request");
            let response = protocol_handle_request(graph, req, session_version).await;
            response.map(|r| serde_json::to_value(&r).unwrap_or(json!({})))
        }
        RpcPayload::Batch(reqs) => {
            debug!(project_id = %project_id, scope = %scope, batch_size = reqs.len(), "Handling RPC batch request");
            let mut responses = Vec::with_capacity(reqs.len());
            for req in reqs {
                if let Some(res) = protocol_handle_request(graph, req, session_version.clone()).await {
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
