#!/bin/bash
PROJECT_ID="bench_1000"
URL="http://127.0.0.1:3000/mcp/projects/$PROJECT_ID/shared"

# 1. Insertion Test
echo "Starting 1000 insertions..."
start_ins=$(date +%s%N)
for i in {1..1000}; do
    curl -s -X POST "$URL" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":$i,\"method\":\"mcp_memory_create_entities\",\"params\":{\"entities\":[{\"name\":\"Node_$i\",\"entityType\":\"Bench\",\"observations\":[\"Observation for $i\"]}]}}" > /dev/null
done
end_ins=$(date +%s%N)
ins_time=$(( (end_ins - start_ins) / 1000000 ))

# 2. Search Test
echo "Starting 1000 searches..."
start_search=$(date +%s%N)
for i in {1..1000}; do
    curl -s -X POST "$URL" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":$i,\"method\":\"mcp_memory_search_nodes\",\"params\":{\"query\":\"Node_$i\"}}" > /dev/null
done
end_search=$(date +%s%N)
search_time=$(( (end_search - start_search) / 1000000 ))

# 3. Cleanup
echo "Cleaning up..."
curl -s -X POST "$URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":9999,\"method\":\"mcp_memory_delete_entities\",\"params\":{\"entityNames\":$(printf '\"Node_%s\",' {1..1000} | sed 's/,$//' | sed 's/^/[/' | sed 's/$/]/')}}" > /dev/null

echo "Results for 1000 operations:"
echo "Insertions: $ins_time ms (avg $(( ins_time / 1000 )) ms/req)"
echo "Searches: $search_time ms (avg $(( search_time / 1000 )) ms/req)"
