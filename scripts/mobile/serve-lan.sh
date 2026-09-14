#!/usr/bin/env bash
# The gateway and one agent, reachable from a phone on the same network.
#
#     scripts/mobile/serve-lan.sh            # start both, print the addresses
#     scripts/mobile/serve-lan.sh --print    # only print the addresses
#
# The gateway already binds 0.0.0.0 (see `baylee-gateway/src/main.rs`), so
# nothing here opens it up that was closed — what this script adds is the
# *address to use from each surface*, which is different for every one of them
# and is the thing that is guessed wrong:
#
#   a phone on WiFi          http://<this machine's LAN IP>:28766
#   a phone over USB         http://127.0.0.1:28766   after `adb reverse`
#   the Android emulator     http://10.0.2.2:28766    (10.0.2.2 is the host)
#   the iOS simulator        http://127.0.0.1:28766   (it shares the host stack)
#
# `gateway-store.json` holds real accounts and is deliberately untracked. It
# is used as-is; nothing here writes it and nothing here may ever stage it.
set -euo pipefail

cd "$(dirname "$0")/../.."
PORT="${PORT:-28766}"
LAN_IP="$(ipconfig getifaddr en0 2>/dev/null || ipconfig getifaddr en1 2>/dev/null || echo '')"

print_addresses() {
    echo
    echo "  gateway port      ${PORT}"
    echo "  this machine      ${LAN_IP:-<no LAN address on en0/en1>}"
    echo
    echo "  phone on WiFi     http://${LAN_IP:-?}:${PORT}"
    echo "  phone over USB    http://127.0.0.1:${PORT}   (adb reverse tcp:${PORT} tcp:${PORT})"
    echo "  Android emulator  http://10.0.2.2:${PORT}"
    echo "  iOS simulator     http://127.0.0.1:${PORT}"
    echo
    echo "  Build the APK against one of those:"
    echo "    scripts/mobile/android-build.sh --gateway http://${LAN_IP:-?}:${PORT}"
    echo
}

if [ "${1:-}" = "--print" ]; then
    print_addresses
    exit 0
fi

if [ ! -x target/debug/baylee-gateway ] || [ ! -x target/debug/baylee-agent ]; then
    echo "building the gateway and the agent first"
    cargo build -p baylee-gateway -p baylee-agent -p baylee-engine-server
fi

# One secret for both halves of this run. A gateway with no agent hosts no
# games — `POST /lobby/games` answers 503 — so the two start together.
TOKEN="${BAYLEE_AGENT_TOKEN:-$(openssl rand -hex 32)}"
mkdir -p /tmp/baylee-lan
export RUST_LOG="${RUST_LOG:-info}"

PORT="$PORT" BAYLEE_AGENT_TOKEN="$TOKEN" ./target/debug/baylee-gateway \
    >/tmp/baylee-lan/gateway.log 2>&1 &
GATEWAY_PID=$!
sleep 1
BAYLEE_GATEWAY="http://127.0.0.1:${PORT}" BAYLEE_AGENT_TOKEN="$TOKEN" \
    BAYLEE_ENGINE_BIN="$PWD/target/debug/baylee-engine-server" \
    ./target/debug/baylee-agent >/tmp/baylee-lan/agent.log 2>&1 &
AGENT_PID=$!

echo "gateway pid ${GATEWAY_PID}, agent pid ${AGENT_PID}"
echo "logs in /tmp/baylee-lan/"
print_addresses
echo "stop them with:  kill ${GATEWAY_PID} ${AGENT_PID}"
wait
