import asyncio
import aiohttp
import time
import os
import statistics

# Configuration
URL = os.getenv("MCP_URL", "http://127.0.0.1:3000/mcp/projects/stress_test/shared")
API_KEY = os.getenv("MCP_API_KEY", "")
CONCURRENT_AGENTS = 20
REQUESTS_PER_AGENT = 100

async def test_agent(agent_id):
    headers = {
        "Content-Type": "application/json"
    }
    if API_KEY:
        headers["Authorization"] = f"Bearer {API_KEY}"
    
    latencies = []
    errors = 0
    node_names = []
    
    async with aiohttp.ClientSession(headers=headers) as session:
        for i in range(REQUESTS_PER_AGENT):
            name = f"Stress_Node_{agent_id}_{i}"
            payload = {
                "jsonrpc": "2.0",
                "id": f"agent_{agent_id}_{i}",
                "method": "tools/call",
                "params": {
                    "name": "create_entities",
                    "arguments": {
                        "entities": [{
                            "name": name,
                            "entityType": "Node",
                            "observations": [f"Stress test latency check {time.time()}"]
                        }]
                    }
                }
            }
            
            start_time = time.perf_counter()
            try:
                async with session.post(URL, json=payload) as resp:
                    if resp.status == 200:
                        latencies.append(time.perf_counter() - start_time)
                        node_names.append(name)
                        await resp.json()
                    else:
                        errors += 1
            except Exception:
                errors += 1
                
    return latencies, errors, node_names

async def cleanup(all_nodes):
    if not all_nodes:
        return
    
    print(f"\n🧹 Cleaning up {len(all_nodes)} nodes...")
    headers = {"Content-Type": "application/json"}
    if API_KEY:
        headers["Authorization"] = f"Bearer {API_KEY}"
        
    payload = {
        "jsonrpc": "2.0",
        "id": "cleanup",
        "method": "tools/call",
        "params": {
            "name": "delete_entities",
            "arguments": {
                "entityNames": all_nodes
            }
        }
    }
    
    async with aiohttp.ClientSession(headers=headers) as session:
        async with session.post(URL, json=payload) as resp:
            if resp.status == 200:
                print("✅ Cleanup successful")
            else:
                print(f"❌ Cleanup failed: {resp.status}")

async def main():
    print(f"🚀 Starting Stress Test: {CONCURRENT_AGENTS} agents, {REQUESTS_PER_AGENT} reqs each")
    print(f"🔗 Target: {URL}")
    if API_KEY:
        print("🔑 Auth: API Key enabled")
    
    start = time.perf_counter()
    results = await asyncio.gather(*[test_agent(i) for i in range(CONCURRENT_AGENTS)])
    total_time = time.perf_counter() - start
    
    all_latencies = []
    total_errors = 0
    all_created_nodes = []
    
    for lats, errs, nodes in results:
        all_latencies.extend(lats)
        total_errors += errs
        all_created_nodes.extend(nodes)
    
    total_reqs = len(all_latencies) + total_errors
    rps = total_reqs / total_time
    
    print("\n" + "="*40)
    print("📊 STRESS TEST RESULTS")
    print("="*40)
    print(f"Total Requests: {total_reqs}")
    print(f"Successful:     {len(all_latencies)}")
    print(f"Failed:         {total_errors}")
    print(f"Total Time:     {total_time:.2f}s")
    print(f"Throughput:     {rps:.2f} req/s")
    
    if all_latencies:
        print("-" * 40)
        print(f"Avg Latency:    {statistics.mean(all_latencies)*1000:.2f} ms")
        print(f"P95 Latency:    {statistics.quantiles(all_latencies, n=20)[18]*1000:.2f} ms")
    print("="*40)

    # Final cleanup
    await cleanup(all_created_nodes)

if __name__ == "__main__":
    asyncio.run(main())
