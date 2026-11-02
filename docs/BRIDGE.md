# Agent bridge

The bridge lets the phone read and message terminal agents. It runs
on the host. It binds 127.0.0.1 only. The app reaches it through a
local proxy. Code lives in `crates/agent-bridge`.


## Endpoints

- `GET /v1/panes`: allowlisted panes with label, kind, and status.
- `GET /v1/panes/{id}/output?lines=N`: last N lines, redacted. N tops at 200.
- `GET /v1/panes/{id}/message/preview?text=...`: returns a one-time code
  plus the exact text. The code lives 60 seconds and works once.
- `POST /v1/panes/{id}/message`: sends text with a live code. Tops at 2000 chars.

## Safeguards

- Own token file. The server refuses to start without it. Compare runs
  in constant time. The token never logs.
- Allowlist file. Empty by default. Empty means nothing is reachable.
  Sends need both the pane id and its label in the list.
- One-time codes. Preview text equals sent text byte for byte.
- Rate limit. Ten sends per minute. Excess answers 429 with retry time.
- Known kinds only. The public table ships Muse plus generic signs.
  Owners add more kinds with `AGENT_BRIDGE_EXTRA_KINDS`.
- Short answers (`y`, `n`, Enter) send only at a visible approval prompt.
- Enter is the only key ever sent. Panes never close, kill, or spawn here.
- Token-looking runs in output become `[redacted]`.
- Audit log. Each send attempt appends time, pane, sender, text
  length, and result. Text never logs.

## Run it

```bash
scripts/agent-bridge.sh setup
scripts/agent-bridge.sh allow w1:p1 my-label
scripts/agent-bridge.sh start
```

Set `AGENT_BRIDGE_PORT` to move off the default. Set
`AGENT_BRIDGE_ORIGINS` to allow more web origins. Never use star.

## Kind table note

The public edition ships a small table on purpose. It proves the
pattern without naming tools you may not use. Add your own agents'
kinds at deploy. Generic yes/no signs cover most approval prompts.
