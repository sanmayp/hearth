#!/usr/bin/env bash
# Hearth demo. Mock data only. No keys leave the machine.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== 1. Route manifest (no database needed) =="
ROUTES="$(cargo run -q -p hearth-server -- --print-routes)"
printf '%.400s\n...\n' "$ROUTES"

echo "== 2. Bridge refuses to start without its token file =="
AGENT_BRIDGE_TOKEN_FILE=/nonexistent-token cargo run -q -p agent-bridge 2>&1 || true

echo "== 3. Baseten reader in mock mode (no key, no network) =="
node tools/baseten-reader/reader.mjs --mock fixtures/filing.txt

echo "Demo done. See README.md for the full quickstart."
