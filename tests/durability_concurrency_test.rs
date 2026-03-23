use reqwest::Client;
mod common;

#[tokio::test]
async fn test_persistence_restart() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/mcp/projects/persist_test/shared", port);

    // 1. Start server and write data
    {
        // Start with 1s save interval using our robust helper
        let _server = env.start_server_with_args(port, None, vec!["--interval", "1"]).await;
        
        let res = client.get(&url).send().await.unwrap();
        let sid = res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();

        let payload = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": "create_entities",
                "arguments": {
                    "entities": [{"name": "Permanent", "entityType": "Statue", "observations": ["Solid"]}]
                }
            }
        });
        client.post(format!("{}?session_id={}", url, sid)).json(&payload).send().await.unwrap();
        
        // Wait for background save (interval is 1s)
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        // _server is dropped here and killed
    }

    // Give OS time to free port
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 2. Restart server and verify data
    {
        let _server = env.start_server(port, None).await;
        
        let res = client.get(&url).send().await.unwrap();
        let sid = res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();

        let payload = serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "read_graph", "arguments": {} }
        });
        let res = client.post(format!("{}?session_id={}", url, sid)).json(&payload).send().await.unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        
        let text = body["result"]["content"][0]["text"].as_str().unwrap_or_else(|| panic!("Body was: {}", body));
        assert!(text.contains("Permanent"), "Expected 'Permanent' in response, got: {}", text);
    }
}

#[tokio::test]
async fn test_concurrency_stress() {
    let env = common::TestEnv::new();
    let port = common::TestEnv::get_free_port();
    let mut _server = env.start_server(port, None).await;
    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{}/mcp/projects/concurrent/shared", port);

    let mut tasks = vec![];
    for i in 0..10 {
        let client_clone = client.clone();
        let url = base_url.clone();
        tasks.push(tokio::spawn(async move {
            // Each agent gets its own session
            let res = client_clone.get(&url).send().await.unwrap();
            let sid = res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();
            
            let payload = serde_json::json!({
                "jsonrpc": "2.0", "id": i, "method": "tools/call",
                "params": {
                    "name": "create_entities",
                    "arguments": {
                        "entities": [{"name": format!("Node_{}", i), "entityType": "Test", "observations": []}]
                    }
                }
            });
            client_clone.post(format!("{}?session_id={}", url, sid)).json(&payload).send().await.unwrap();
        }));
    }

    for t in tasks {
        t.await.unwrap();
    }

    // Use a fresh session to read back
    let res = client.get(&base_url).send().await.unwrap();
    let sid = res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();

    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 100, "method": "tools/call",
        "params": { "name": "read_graph", "arguments": {} }
    });
    let res = client.post(format!("{}?session_id={}", base_url, sid)).json(&payload).send().await.unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    let text = body["result"]["content"][0]["text"].as_str().unwrap_or_else(|| panic!("Body was: {}", body));
    
    for i in 0..10 {
        assert!(text.contains(&format!("Node_{}", i)), "Missing Node_{} in {}", i, text);
    }
}
