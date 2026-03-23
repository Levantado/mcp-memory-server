use reqwest::{Client, StatusCode, Method};
mod common;

const TEST_KEY: &str = "test-secret-key";

#[tokio::test]
async fn test_handshake_matrix() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let mut _server = env.start_server(port, None).await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // Test cases: (version_to_send, expected_status)
    let cases = [
        ("2024-11-05", StatusCode::OK),
        ("2025-11-25", StatusCode::OK),
        ("invalid-version", StatusCode::OK), // Should default to latest or accept
    ];

    for (version, status) in cases {
        let res = client.get(&base_url)
            .header("mcp-protocol-version", version)
            .send().await.unwrap();
        
        assert_eq!(res.status(), status, "Failed for version: {}", version);
        assert_eq!(res.headers().get("content-type").unwrap(), "text/event-stream");
        // Server should respond with a valid version
        let resp_version = res.headers().get("mcp-protocol-version").unwrap();
        assert!(resp_version == "2025-11-25" || resp_version == version);
    }
}

#[tokio::test]
async fn test_auth_edge_cases() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let mut _server = env.start_server(port, Some(TEST_KEY)).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // 1. Missing Authorization
    assert_eq!(client.get(&url).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);

    // 2. Empty Bearer
    assert_eq!(client.get(&url).header("Authorization", "Bearer ").send().await.unwrap().status(), StatusCode::UNAUTHORIZED);

    // 3. Wrong prefix
    assert_eq!(client.get(&url).header("Authorization", format!("Basic {}", TEST_KEY)).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);

    // 4. Case sensitivity check (headers names are case-insensitive in HTTP, but value must match)
    assert_eq!(client.get(&url).header("authorization", format!("bearer {}", TEST_KEY)).send().await.unwrap().status(), StatusCode::UNAUTHORIZED); // "bearer" vs "Bearer"
    
    // 5. Valid access
    assert_eq!(client.get(&url).header("Authorization", format!("Bearer {}", TEST_KEY)).send().await.unwrap().status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cors_preflight() {
    // Start server with specific origins
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_mcp-memory-server-rust"));
    cmd.arg("--mode").arg("http")
       .arg("--port").arg(port.to_string())
       .arg("--root").arg(env.storage_path.to_str().unwrap())
       .arg("--cors-allowed-origins").arg("https://trusted.com,http://localhost:8080");
    
    cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    let mut server = cmd.spawn().unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // 1. Trusted origin
    let res = client.request(Method::OPTIONS, &url)
        .header("Origin", "https://trusted.com")
        .header("Access-Control-Request-Method", "POST")
        .send().await.unwrap();
    assert_eq!(res.headers().get("access-control-allow-origin").unwrap(), "https://trusted.com");

    // 2. Untrusted origin
    let res = client.request(Method::OPTIONS, &url)
        .header("Origin", "https://malicious.com")
        .header("Access-Control-Request-Method", "POST")
        .send().await.unwrap();
    assert!(res.headers().get("access-control-allow-origin").is_none());
}
