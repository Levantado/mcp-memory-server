import asyncio
import aiohttp
import time

async def test_agent(agent_id, project_id):
    url = f"http://127.0.0.1:3000/projects/{project_id}/shared"
    async with aiohttp.ClientSession() as session:
        for i in range(50):
            payload = {
                "jsonrpc": "2.0",
                "id": i,
                "method": "mcp_memory_create_entities",
                "params": {
                    "entities": [{
                        "name": f"Agent_{agent_id}_Node_{i}",
                        "entity_type": "Node",
                        "observations": ["Stress test node"]
                    }]
                }
            }
            async with session.post(url, json=payload) as resp:
                await resp.json()

async def main():
    tasks = [test_agent(i, "stress_test") for i in range(10)]
    start = time.time()
    await asyncio.gather(*tasks)
    print(f"Stress test finished in {time.time() - start:.2f}s")

if __name__ == "__main__":
    asyncio.run(main())
