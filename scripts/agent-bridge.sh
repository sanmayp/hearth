#!/usr/bin/env bash
# Agent bridge helper. The owner runs this on the host.
# It sets the token file and the allowlist, then starts the server.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BRIDGE_HOME="${BRIDGE_HOME:-$HOME/.config/agent-bridge}"
TOKEN_FILE="$BRIDGE_HOME/token"
ALLOW_FILE="$BRIDGE_HOME/allowlist"
AUDIT_FILE="$BRIDGE_HOME/audit.jsonl"
PORT="${AGENT_BRIDGE_PORT:-8449}"
TAILNET="https://example.ts.net:8449"
BIN="$ROOT/target/release/agent-bridge"

usage() {
  cat <<'EOF'
Usage: agent-bridge.sh <setup|start|check|allow> [args]

  setup [--no-build]     Make config dir, token, allowlist, audit file, binary.
  start                  Run the server in front. Loopback only.
  check                  Probe health and list reachable panes.
  allow <pane-id> <label>  Add one pane id plus label to the allowlist.
EOF
}

need_token() {
  if [ ! -s "$TOKEN_FILE" ]; then
    echo "Token file is missing. Run setup first."
    exit 1
  fi
}

cmd_setup() {
  local build=1
  if [ "${1:-}" = "--no-build" ]; then
    build=0
  fi
  mkdir -p "$BRIDGE_HOME"
  chmod 700 "$BRIDGE_HOME"
  if [ -s "$TOKEN_FILE" ]; then
    echo "Token file stays. It already exists."
  else
    if command -v openssl >/dev/null 2>&1; then
      openssl rand -hex 32 >"$TOKEN_FILE"
    else
      od -An -tx1 -N32 /dev/urandom | tr -d ' \n' >"$TOKEN_FILE"
    fi
    chmod 600 "$TOKEN_FILE"
    echo "Token file is new."
  fi
  if [ ! -f "$ALLOW_FILE" ]; then
    : >"$ALLOW_FILE"
    echo "Allowlist is new and empty. Nothing is reachable yet."
  else
    echo "Allowlist stays. It already exists."
  fi
  if [ ! -f "$AUDIT_FILE" ]; then
    : >"$AUDIT_FILE"
  fi
  if [ "$build" = "1" ]; then
    (cd "$ROOT" && cargo build --release -p agent-bridge)
  fi
  echo "Setup is done. Add panes, then run start."
  echo "Find panes with: herdr pane list"
}

cmd_start() {
  need_token
  if [ ! -x "$BIN" ]; then
    echo "Binary is missing. Run setup first."
    exit 1
  fi
  echo "Bridge serves on 127.0.0.1:$PORT."
  AGENT_BRIDGE_TOKEN_FILE="$TOKEN_FILE" \
    AGENT_BRIDGE_ALLOWLIST="$ALLOW_FILE" \
    AGENT_BRIDGE_AUDIT_LOG="$AUDIT_FILE" \
    AGENT_BRIDGE_PORT="$PORT" \
    exec "$BIN"
}

cmd_check() {
  need_token
  local token
  token="$(cat "$TOKEN_FILE")"
  echo "Health:"
  curl -sf "http://127.0.0.1:$PORT/health/live" && echo
  echo "Panes:"
  curl -sf -H "Authorization: Bearer $token" \
    "http://127.0.0.1:$PORT/v1/panes" && echo
  echo "Tailnet:"
  if ! curl -sf -o /dev/null -H "Authorization: Bearer $token" "$TAILNET/v1/panes"; then
    echo "Tailnet panes call failed. Check the proxy."
    exit 1
  fi
  echo "Tailnet panes OK."
  echo "Preflight:"
  local resp
  if ! resp=$(curl -s -D - -o /dev/null --max-time 10 -X OPTIONS \
    -H "Origin: http://tauri.localhost" \
    -H "Access-Control-Request-Method: POST" \
    "$TAILNET/v1/panes"); then
    echo "Preflight call failed. Check the bridge and the proxy."
    exit 1
  fi
  local code hdr
  code=$(printf '%s\n' "$resp" | head -n 1 | awk '{print $2}')
  hdr=$(printf '%s\n' "$resp" | grep -i '^access-control-allow-origin:' | tr -d '\r' | awk '{print $2}')
  if [ "$code" != "204" ]; then
    echo "Preflight failed: want 204, got ${code:-none}."
    exit 1
  fi
  if [ "$hdr" != "http://tauri.localhost" ]; then
    echo "Preflight origin is not echoed."
    exit 1
  fi
  echo "Preflight OK: 204, origin echoed."
}

cmd_allow() {
  if [ "$#" -ne 2 ]; then
    echo "Usage: agent-bridge.sh allow <pane-id> <label>"
    exit 1
  fi
  local id="$1" label="$2"
  if ! [[ "$id" =~ ^w[0-9]+:p[0-9A-Za-z]+$ ]]; then
    echo "Pane id is bad. Shape is w1:p19."
    exit 1
  fi
  if [ -z "$label" ]; then
    echo "Label is empty."
    exit 1
  fi
  mkdir -p "$BRIDGE_HOME"
  touch "$ALLOW_FILE"
  for line in "$id" "$label"; do
    if ! grep -qxF "$line" "$ALLOW_FILE"; then
      printf '%s\n' "$line" >>"$ALLOW_FILE"
    fi
  done
  echo "Pane is allowed: $id ($label)."
}

cmd="${1:-help}"
case "$cmd" in
  setup) shift; cmd_setup "$@" ;;
  start) cmd_start ;;
  check) cmd_check ;;
  allow) shift; cmd_allow "$@" ;;
  *) usage ;;
esac
