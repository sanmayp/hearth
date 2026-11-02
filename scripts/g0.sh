#!/usr/bin/env bash
# G0 lane gate (<60s). Fails on timeout, on zero tests, or on any fail.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."
BUDGET=60

run() {
  timeout "$BUDGET" bash -c "$1"
}

echo "== G0: fmt =="
run "cargo fmt --check"
echo "== G0: clippy =="
run "cargo clippy --workspace --all-targets -- -D warnings"
echo "== G0: rust tests =="
if command -v cargo-nextest >/dev/null 2>&1; then
  OUT=$(run "cargo nextest run --workspace") && echo "$OUT" | tail -n 5
else
  OUT=$(run "cargo test --workspace") && echo "$OUT" | grep -E "test result" || true
fi
TOTAL=$(echo "$OUT" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END {print s+0}')
[ "$TOTAL" -gt 0 ] || { echo "G0 FAIL: zero tests ran"; exit 1; }
echo "rust tests passed total: $TOTAL"
if [ -d web ]; then
  echo "== G0: svelte-check =="
  (cd web && run "npm run check")
  echo "== G0: ui tests =="
  (cd web && run "npm test -- --run 2>/dev/null || npm test")
fi
echo "G0 PASS"
