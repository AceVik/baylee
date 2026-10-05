#!/bin/bash
set -e
export DATABASE_URL="postgres://baylee:baylee@127.0.0.1:5432/baylee"
export BAYLEE_AGENT_TOKEN="dev-token"
export PORT="28766"

# Start gateway
./target/debug/baylee-gateway > gateway.log 2>&1 &
GW_PID=$!
echo "Started gateway (PID $GW_PID)"

# Wait for gateway to be healthy
for i in {1..30}; do
    if curl -s http://127.0.0.1:28766/health | grep -q '"database":true'; then
        echo "Gateway is ready"
        break
    fi
    sleep 0.2
done

# Start agent
export BAYLEE_GATEWAY="http://127.0.0.1:28766"
export BAYLEE_ENGINE_BIN="$(pwd)/target/debug/baylee-engine-server"
./target/debug/baylee-agent > agent.log 2>&1 &
AGENT_PID=$!
echo "Started agent (PID $AGENT_PID)"

# Wait for agent to connect to gateway
for i in {1..30}; do
    if curl -s http://127.0.0.1:28766/health | grep -q '"connected":1'; then
        echo "Agent connected to gateway!"
        break
    fi
    sleep 0.2
done
