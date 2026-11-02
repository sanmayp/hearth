#!/usr/bin/env bash
# Fake herdr for agent-bridge tests. It serves canned data.
# It never calls the real herdr. It never touches real panes.
set -u

fail() {
  echo '{"error":{"code":"down","message":"fake herdr is down"}}' >&2
  exit 1
}

if [ "${FAKE_HERDR_FAIL:-0}" = "1" ]; then
  fail
fi

cmd="${1:-}"
sub="${2:-}"

if [ "$cmd" = "pane" ] && [ "$sub" = "list" ]; then
  if [ -n "${FAKE_HERDR_PANES_FILE:-}" ]; then
    cat "$FAKE_HERDR_PANES_FILE"
  else
    cat <<'EOF'
{"result":{"type":"pane_list","panes":[
{"pane_id":"w1:p1","label":"lifeos","tab_id":"w1:t1","agent":"muse","agent_status":"idle"},
{"pane_id":"w1:p2","label":"site","tab_id":"w1:t1","agent":"muse","agent_status":"working"}
]}}
EOF
  fi
  exit 0
fi

if [ "$cmd" = "pane" ] && [ "$sub" = "read" ]; then
  if [ -n "${FAKE_HERDR_READ_FILE:-}" ]; then
    cat "$FAKE_HERDR_READ_FILE"
  else
    cat <<'EOF'
{"result":{"type":"pane_read","pane_id":"w1:p1","workspace_id":"w1","tab_id":"w1:t1","source":"visible","format":"text","text":"line one\nline two\n","revision":1,"truncated":false}}
EOF
  fi
  exit 0
fi

if [ "$cmd" = "pane" ] && [ "$sub" = "send-text" ]; then
  id="${3:-}"
  text="${4:-}"
  printf 'send-text %s %s\n' "$id" "$text" >> "${FAKE_HERDR_SENDLOG:?sendlog is not set}"
  echo '{"result":{"type":"ok"}}'
  exit 0
fi

if [ "$cmd" = "pane" ] && [ "$sub" = "send-keys" ]; then
  id="${3:-}"
  shift 3
  printf 'send-keys %s %s\n' "$id" "$*" >> "${FAKE_HERDR_SENDLOG:?sendlog is not set}"
  echo '{"result":{"type":"ok"}}'
  exit 0
fi

echo '{"error":{"code":"usage","message":"bad fake-herdr call"}}' >&2
exit 2
