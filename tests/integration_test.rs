use reqwest::{Client, StatusCode};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

mod common;

const TEST_KEY: &str = "test-secret-key";

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
    let port = common::TestEnv::get_free_port();
    
    let mut server = start_test_server(&env, port, Some(TEST_KEY)).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // 1. Request without token should fail
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Request with valid token should succeed
    let res = client.get(&url).header("Authorization", format!("Bearer {}", TEST_KEY)).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    server.kill().await.unwrap();
}

#[tokio::test]
async fn test_sse_handshake() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let mut server = start_test_server(&env, port, Some(TEST_KEY)).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test_proj/shared", port);

    let res = client.get(&url)
        .header("Authorization", format!("Bearer {}", TEST_KEY))
        .send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    
    // Check required headers
    assert!(res.headers().contains_key("mcp-session-id"));
    assert_eq!(res.headers().get("mcp-protocol-version").unwrap(), "2025-11-25");

    server.kill().await.unwrap();
}

#[tokio::test]
async fn test_resource_reading() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let mut server = start_test_server(&env, port, Some(TEST_KEY)).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // Get session ID via GET
    let get_res = client.get(&url)
        .header("Authorization", format!("Bearer {}", TEST_KEY))
        .send().await.unwrap();
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
        .header("Authorization", format!("Bearer {}", TEST_KEY))
        .json(&payload)
        .send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    
    let text_content = body.get("result")
        .and_then(|r| r.get("contents"))
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or_else(|| panic!("Expected text content, got: {}", body));
    
    assert_eq!(text_content, "test policy");

    server.kill().await.unwrap();
}
