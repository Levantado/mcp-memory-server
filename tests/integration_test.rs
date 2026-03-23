use reqwest::{Client, StatusCode};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

mod common;

async fn start_test_server(env: &common::TestEnv, port: u16, api_key: Option<&str>) -> tokio::process::Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mcp-memory-server-rust"));
    
    cmd.arg("--mode").arg("http")
       .arg("--port").arg(port.to_string())
       .arg("--root").arg(env.storage_path.to_str().unwrap())
       .arg("--docs-dir").arg(env.docs_path.to_str().unwrap());

    if let Some(key) = api_key {
        cmd.arg("--api-key").arg(key);
    }

    cmd.stdout(Stdio::inherit())
       .stderr(Stdio::inherit());

    let mut child = cmd.spawn().expect("Failed to start server");
    
    // Wait for server to boot
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    if let Ok(Some(status)) = child.try_wait() {
        panic!("Server died immediately with status: {}", status);
    }

    child
}

#[tokio::test]
async fn test_auth_flow() {
    let env = common::TestEnv::new();
    let port = 5051;
    let api_key = "test-secret-key";
    
    let mut server = start_test_server(&env, port, Some(api_key)).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // 1. Request without token should fail
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Request with invalid token should fail
    let res = client.get(&url).header("Authorization", "Bearer wrong-key").send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. Request with valid token should succeed
    let res = client.get(&url).header("Authorization", format!("Bearer {}", api_key)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    server.kill().await.unwrap();
}

#[tokio::test]
async fn test_sse_handshake() {
    let env = common::TestEnv::new();
    let port = 5052;
    let mut server = start_test_server(&env, port, None).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test_proj/shared", port);

    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    
    // Check required headers
    assert!(res.headers().contains_key("mcp-session-id"));
    assert_eq!(res.headers().get("mcp-protocol-version").unwrap(), "2025-11-25");
    assert_eq!(res.headers().get("content-type").unwrap(), "text/event-stream");

    // Read the first chunk to ensure the endpoint event is fired
    let mut stream = res.bytes_stream();
    use tokio_stream::StreamExt;
    
    let chunk = stream.next().await.expect("Expected data").unwrap();
    let text = String::from_utf8_lossy(&chunk);
    
    assert!(text.contains("event: endpoint"));
    assert!(text.contains("data: http://127.0.0.1:5052/mcp/projects/test_proj/shared?session_id="));

    server.kill().await.unwrap();
}

#[tokio::test]
async fn test_resource_reading() {
    let env = common::TestEnv::new();
    let port = 5053;
    let mut server = start_test_server(&env, port, None).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // Get session ID via GET
    let get_res = client.get(&url).send().await.unwrap();
    let session_id = get_res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();

    // Make a POST request to read the resource
    let post_url = format!("{}?session_id={}", url, session_id);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "1",
        "method": "resources/read",
        "params": {
            "uri": "mcp://resources/guidelines/effective_work"
        }
    });

    let res = client.post(&post_url)
        .json(&payload)
        .send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    
    // Check if the content was successfully read from our mock env
    let text_content = body.get("result")
        .and_then(|r| r.get("contents"))
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or_else(|| panic!("Expected text content, got: {}", body));
    
    assert_eq!(text_content, "test policy"); // Matches what we wrote in TestEnv

    server.kill().await.unwrap();
}
