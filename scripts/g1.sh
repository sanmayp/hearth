#!/usr/bin/env bash
# G1 merge train (<4min). Full G0, contract diff, migration dry-run, UI build.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."
BUDGET=240

echo "== G1: full G0 (with throwaway PG for migration test) =="
test -n "${PG_DRYRUN_URL:-}" || { echo "G1 FAIL: set PG_DRYRUN_URL (throwaway Postgres)"; exit 1; }
test -n "${REDIS_DRYRUN_URL:-}" || { echo "G1 FAIL: set REDIS_DRYRUN_URL (throwaway Redis)"; exit 1; }
export TEST_PG_URL="$PG_DRYRUN_URL"
timeout "$BUDGET" bash scripts/g0.sh
unset TEST_PG_URL

echo "== G1: contract snapshot =="
test -f contracts/routes.snapshot.json || { echo "G1 FAIL: missing snapshot"; exit 1; }
# Manifest must stay sorted and keep all snapshot routes (additive only).
cargo run -q -p hearth-server -- --print-routes 2>/dev/null \
  | diff -u contracts/routes.snapshot.json - \
  || { echo "G1 FAIL: contract drift (additive only, update snapshot on purpose)"; exit 1; }

echo "== G1: migration dry-run (throwaway PG + Redis) =="
DATABASE_URL="$PG_DRYRUN_URL" REDIS_URL="$REDIS_DRYRUN_URL" timeout 120 cargo run -q -p hearth-server -- --migrate-only

echo "== G1: UI build =="
(cd web && timeout 180 npm run build)

echo "G1 PASS"
