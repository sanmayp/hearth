#!/usr/bin/env bash
# Heavy AI demo. Dry-run by default. Live needs keys plus caps.
# Mock data only unless --live runs with owner keys in env.
set -euo pipefail
cd "$(dirname "$0")/.."

LIVE=0
MOCK_RUN=0
PROVIDER="mock"
TASK="summary"
INPUT="fixtures/ai-input"
MAX_CALLS="${HEARTH_AI_MAX_CALLS:-200}"
MAX_TOKENS="${HEARTH_AI_MAX_TOKENS:-500000}"

for arg in "$@"; do
  case "$arg" in
    --live) LIVE=1 ;;
    --mock-run) MOCK_RUN=1 ;;
    --provider=*) PROVIDER="${arg#--provider=}" ;;
    --task=*) TASK="${arg#--task=}" ;;
    --input=*) INPUT="${arg#--input=}" ;;
    --max-calls=*) MAX_CALLS="${arg#--max-calls=}" ;;
    --max-tokens=*) MAX_TOKENS="${arg#--max-tokens=}" ;;
    *) echo "Bad arg: $arg"; exit 1 ;;
  esac
done

if [ "$LIVE" = "1" ] && [ "$PROVIDER" = "mock" ]; then
  echo "Live needs --provider=baseten, modal, or both. Mock stays mock."
  exit 1
fi

if [ "$LIVE" = "0" ]; then
  echo "== Dry run (no calls, no spend) =="
  cargo run -q -p ai-batch -- run --input="$INPUT" --provider="$PROVIDER" \
    --task="$TASK" --max-calls="$MAX_CALLS" --max-tokens="$MAX_TOKENS" --dry-run
fi

if [ "$MOCK_RUN" = "1" ]; then
  echo "== Mock batch (labelled mock, no keys, no network) =="
  cargo run -q -p ai-batch -- run --input="$INPUT" --provider=mock \
    --task="$TASK" --max-calls="$MAX_CALLS" --max-tokens="$MAX_TOKENS"
fi

if [ "$LIVE" = "1" ]; then
  echo "== Live batch (capped, metered) =="
  cargo run -q -p ai-batch -- run --input="$INPUT" --provider="$PROVIDER" \
    --task="$TASK" --max-calls="$MAX_CALLS" --max-tokens="$MAX_TOKENS"
fi

if [ "$MOCK_RUN" = "1" ] || [ "$LIVE" = "1" ]; then
  echo "== Usage report (generated from usage.jsonl) =="
  cargo run -q -p ai-batch -- report \
    --usage=out/ai-batch/usage.jsonl --out=docs/USAGE-REPORT.md
  cat docs/USAGE-REPORT.md
elif [ -f docs/USAGE-REPORT.md ]; then
  echo "== Last report (dry run changes nothing) =="
  cat docs/USAGE-REPORT.md
else
  echo "No report yet. Run with --mock-run or --live to make one."
fi
echo "Demo done."
