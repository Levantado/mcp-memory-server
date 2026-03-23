#!/bin/bash
PROJECT_ID="stress_test"
URL="http://127.0.0.1:3000/projects/$PROJECT_ID/shared"

run_agent() {
    AGENT_ID=$1
    for i in {1..20}; do
        curl -s -X POST "$URL" \
            -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"id\":$i,\"method\":\"mcp_memory_create_entities\",\"params\":{\"entities\":[{\"name\":\"Agent_${AGENT_ID}_Node_${i}\",\"entity_type\":\"Node\",\"observations\":[\"Stress test node\"]}]}}" > /dev/null
    done
}

export -f run_agent
export URL

start=$(date +%s%N)
for i in {1..10}; do
    run_agent $i &
done

wait
end=$(date +%s%N)
runtime=$(( (end - start) / 1000000 ))
echo "Stress test finished in $runtime ms"
