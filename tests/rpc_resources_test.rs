use reqwest::{Client, StatusCode};
mod common;

#[tokio::test]
async fn test_batch_rpc() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let mut _server = env.start_server(port, None).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    // 1. Get session ID
    let res = client.get(&url).send().await.unwrap();
    let sid = res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();

    // 2. Send mixed batch: valid + invalid + notification
    let payload = serde_json::json!([
        { "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} },
        { "jsonrpc": "2.0", "id": 2, "method": "invalid_method", "params": {} },
        { "jsonrpc": "2.0", "method": "notifications/initialized", "params": {} } // Notification (no id)
    ]);

    let res = client.post(format!("{}?session_id={}", url, sid))
        .json(&payload)
        .send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    
    assert!(body.is_array());
    let results = body.as_array().unwrap();
    assert_eq!(results.len(), 2); // Notifications don't get responses
    
    // Check first result (success)
    assert!(results.iter().any(|r| r["id"] == 1 && r.get("result").is_some()));
    // Check second result (error)
    assert!(results.iter().any(|r| r["id"] == 2 && r.get("error").is_some()));
}

#[tokio::test]
async fn test_resource_robustness() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let mut _server = env.start_server(port, None).await;
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/test/shared", port);

    let res = client.get(&url).send().await.unwrap();
    let sid = res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();

    // 1. Test invalid URI
    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "resources/read", "params": { "uri": "mcp://invalid" }
    });
    let res = client.post(format!("{}?session_id={}", url, sid)).json(&payload).send().await.unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["result"]["contents"][0]["text"].as_str().unwrap().contains("Resource not found"));

    // 2. Test project list
    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "resources/list", "params": {}
    });
    let res = client.post(format!("{}?session_id={}", url, sid)).json(&payload).send().await.unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    let resources = body["result"]["resources"].as_array().unwrap();
    assert!(resources.iter().any(|r| r["uri"] == "mcp://projects/dummy_project/raw"));
}
