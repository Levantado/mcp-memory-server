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
    
    async with aiohttp.ClientSession(headers=headers) as session:
        for i in range(REQUESTS_PER_AGENT):
            payload = {
                "jsonrpc": "2.0",
                "id": f"agent_{agent_id}_{i}",
                "method": "tools/call",
                "params": {
                    "name": "create_entities",
                    "arguments": {
                        "entities": [{
                            "name": f"Stress_Node_{agent_id}_{i}",
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
                        await resp.json()
                    else:
                        errors += 1
                        # print(f"Error {resp.status}: {await resp.text()}")
            except Exception as e:
                errors += 1
                # print(f"Request failed: {e}")
                
    return latencies, errors

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
    for lats, errs in results:
        all_latencies.extend(lats)
        total_errors += errs
    
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
        print(f"Min Latency:    {min(all_latencies)*1000:.2f} ms")
        print(f"Max Latency:    {max(all_latencies)*1000:.2f} ms")
        print(f"P95 Latency:    {statistics.quantiles(all_latencies, n=20)[18]*1000:.2f} ms")
    print("="*40)

if __name__ == "__main__":
    asyncio.run(main())
