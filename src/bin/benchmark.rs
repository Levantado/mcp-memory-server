use reqwest::{Client, header};
use serde_json::json;
use std::env;
use std::time::{Duration, Instant};

const CONCURRENT_AGENTS: usize = 20;
const REQUESTS_PER_AGENT: usize = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::var("MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/mcp/projects/stress_test/shared".to_string());
    let api_key = env::var("MCP_API_KEY").unwrap_or_default();

    println!("🚀 Starting Rust Benchmark: {} agents, {} reqs each", CONCURRENT_AGENTS, REQUESTS_PER_AGENT);
    println!("🔗 Target: {}", url);
    if !api_key.is_empty() {
        println!("🔑 Auth: API Key enabled");
    }

    // Configure connection pool
    let mut headers = header::HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    if !api_key.is_empty() {
        let auth_val = format!("Bearer {}", api_key);
        headers.insert(header::AUTHORIZATION, auth_val.parse().unwrap());
    }

    let client = Client::builder()
        .default_headers(headers)
        .pool_idle_timeout(Some(Duration::from_secs(30)))
        .pool_max_idle_per_host(CONCURRENT_AGENTS)
        .build()?;

    // Pre-flight check
    println!("🔍 Checking server reachability...");
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => println!("✅ Server is reachable"),
        Ok(r) => {
            eprintln!("❌ Server returned error: {}. Check your MCP_API_KEY.", r.status());
            return Ok(());
        }
        Err(e) => {
            eprintln!("❌ Could not reach server: {}. Make sure it is running on 127.0.0.1:3000", e);
            return Ok(());
        }
    }

    let mut tasks = Vec::with_capacity(CONCURRENT_AGENTS);
    let start_total = Instant::now();

    for agent_id in 0..CONCURRENT_AGENTS {
        let client_clone = client.clone();
        let url_clone = url.clone();

        tasks.push(tokio::spawn(async move {
            let mut latencies = Vec::with_capacity(REQUESTS_PER_AGENT);
            let mut errors = 0;
            let mut node_names = Vec::with_capacity(REQUESTS_PER_AGENT);

            // Create a session for this agent
            let res = match client_clone.get(&url_clone).send().await {
                Ok(r) => r,
                Err(_) => {
                    return (latencies, REQUESTS_PER_AGENT, node_names);
                }
            };
            
            let sid = if let Some(header_val) = res.headers().get("mcp-session-id") {
                header_val.to_str().unwrap_or("").to_string()
            } else {
                return (latencies, REQUESTS_PER_AGENT, node_names);
            };
            
            let post_url = format!("{}?session_id={}", url_clone, sid);

            for req_id in 0..REQUESTS_PER_AGENT {
                let name = format!("Bench_Node_{}_{}", agent_id, req_id);
                let payload = json!({
                    "jsonrpc": "2.0",
                    "id": format!("req_{}", req_id),
                    "method": "tools/call",
                    "params": {
                        "name": "create_entities",
                        "arguments": {
                            "entities": [{
                                "name": name.clone(),
                                "entityType": "Benchmark",
                                "observations": ["Rust client benchmark"]
                            }]
                        }
                    }
                });

                let start_req = Instant::now();
                match client_clone.post(&post_url).json(&payload).send().await {
                    Ok(r) if r.status().is_success() => {
                        latencies.push(start_req.elapsed());
                        node_names.push(name);
                        let _ = r.bytes().await; // Consume body
                    }
                    _ => errors += 1,
                }
            }
            (latencies, errors, node_names)
        }));
    }

    let mut all_latencies = Vec::with_capacity(CONCURRENT_AGENTS * REQUESTS_PER_AGENT);
    let mut total_errors = 0;
    let mut all_nodes = Vec::with_capacity(CONCURRENT_AGENTS * REQUESTS_PER_AGENT);

    for t in tasks {
        if let Ok((lats, errs, nodes)) = t.await {
            all_latencies.extend(lats);
            total_errors += errs;
            all_nodes.extend(nodes);
        }
    }

    let total_time = start_total.elapsed();
    let total_reqs = all_latencies.len() + total_errors;
    let rps = total_reqs as f64 / total_time.as_secs_f64();

    println!("\n========================================");
    println!("📊 RUST BENCHMARK RESULTS");
    println!("========================================");
    println!("Total Requests: {}", total_reqs);
    println!("Successful:     {}", all_latencies.len());
    println!("Failed:         {}", total_errors);
    println!("Total Time:     {:.2?}", total_time);
    println!("Throughput:     {:.2} req/s", rps);

    if !all_latencies.is_empty() {
        all_latencies.sort_unstable();
        let sum: Duration = all_latencies.iter().sum();
        let avg = sum / all_latencies.len() as u32;
        let p50 = all_latencies[all_latencies.len() / 2];
        let p90 = all_latencies[(all_latencies.len() as f64 * 0.9) as usize];
        let p99 = all_latencies[(all_latencies.len() as f64 * 0.99) as usize];

        println!("----------------------------------------");
        println!("Avg Latency:    {:.2?}", avg);
        println!("Min Latency:    {:.2?}", all_latencies[0]);
        println!("Max Latency:    {:.2?}", all_latencies.last().unwrap());
        println!("P50 Latency:    {:.2?}", p50);
        println!("P90 Latency:    {:.2?}", p90);
        println!("P99 Latency:    {:.2?}", p99);
    }
    println!("========================================\n");

    // Cleanup Phase
    if !all_nodes.is_empty() {
        println!("🧹 Cleaning up {} nodes...", all_nodes.len());
        
        let mut cleanup_errors = 0;
        
        // Batch cleanup in chunks of 500 to avoid giant payloads
        for chunk in all_nodes.chunks(500) {
            let res = client.get(&url).send().await?;
            let sid = res.headers().get("mcp-session-id").unwrap().to_str().unwrap().to_string();
            let post_url = format!("{}?session_id={}", url, sid);

            let payload = json!({
                "jsonrpc": "2.0",
                "id": "cleanup",
                "method": "tools/call",
                "params": {
                    "name": "delete_entities",
                    "arguments": {
                        "entityNames": chunk
                    }
                }
            });

            match client.post(&post_url).json(&payload).send().await {
                Ok(r) if r.status().is_success() => {}
                _ => cleanup_errors += 1,
            }
        }
        
        if cleanup_errors == 0 {
            println!("✅ Cleanup successful");
        } else {
            println!("❌ Cleanup had {} failed batches", cleanup_errors);
        }
    }

    Ok(())
}
