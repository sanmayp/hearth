# Agent bridge (design, S3 review)

Status: built. This file records the approved design.

Goal: the owner sees and messages coding agents from the phone. Agents run in herdr panes on the host. The bridge stays agent-agnostic. It works with Muse and future agents in panes.

## Shape

The bridge is a small separate Rust binary, `agent-bridge`, in `crates/agent-bridge`. It is not a route inside hearth-server. It runs on the host under systemd user. It listens on 127.0.0.1 only, port 8449 by default (`AGENT_BRIDGE_PORT` changes it). The phone reaches it through the tailnet proxy. This matches the Hearth route pattern.

## Auth

The bridge has its own token. It is not `HEARTH_TOKEN`. The token lives in a file on the host. The server reads it at start. It refuses to start without the file. Token compare runs in constant time. No token ever ships in the app or the repo.

## Backend

The bridge talks only to the herdr CLI. It uses fixed calls with fixed arguments:

- `herdr pane list`
- `herdr pane read <id> --source visible`
- `herdr pane send-text <id> <text>`
- `herdr pane send-keys <id> Enter`
- `herdr pane send-keys <id> Escape` (Escape key only, never with Enter)

Nothing else runs. No shell interpolation exists. Arguments pass as a list, never through a shell string. Pane ids must match `^w[0-9]+:p[0-9A-Za-z]+$`. Anything else is rejected before any call.

## Endpoints

- `GET /v1/panes`: id, label, tab, agent kind, status. Read-only.
- `GET /v1/panes/{id}/output?lines=N`: last N lines, N at most 200. Read-only. Token-looking strings are redacted before the response leaves the bridge.
- `GET /v1/panes/{id}/message/preview?text=...`: checks a draft and returns a one-time confirm code plus the exact text that will be sent. The code expires after 60 seconds. One code works once.
- `POST /v1/panes/{id}/message`: sends text up to 2000 chars. Needs the live confirm code from preview. Rejects without it.

## Reply flow

1. The owner opens a pane on the phone and reads its output.
2. The owner types a message and taps Preview.
3. The phone shows the exact text that will be sent, plus the target pane name.
4. The owner taps Confirm within 60 seconds. The app sends the text with the code.
5. The bridge checks the code, the allowlist, the rate limit, and the pane status. Then it sends.
6. The app shows the result: sent, refused with a reason, or code expired.

No message sends without the two taps. The preview text is the sent text, byte for byte.

## Session allowlist

A config file on the host lists the pane ids the bridge may touch. Default is empty. Empty means nothing is reachable. The owner adds one id per line to open it. The bridge also checks the pane label against the allowlist on every send. Labels can drift, so both checks run.

## Agent kinds

Agent kind comes from the pane list. Each known kind has a small table of approval-prompt patterns (strings the agent prints when it waits). Unknown kinds get the generic rules only. Sending to an unknown kind is refused until the owner allows that kind in the config.

## Approval prompts

Some agents pause and ask (yes or no, approve, continue). The bridge detects this from the last lines of pane output, using the kind table. Only then may the phone offer the short answers y or n plus Enter. The phone shows the prompt text next to the buttons. No other raw keystrokes ever send.

## Blocked actions

- No pane create, close, or kill.
- No raw keystrokes except Enter, Escape, the literal answers y or n, and the menu digits 1 to 4. Answers and digits send only at a detected approval prompt. Escape sends alone.
- No send to a pane id outside the allowlist.
- No send to a pane whose label is outside the allowlist.
- No send to a pane whose status is unknown.
- No send to an agent kind the owner has not allowed.
- No send without a live confirm code.
- No text over 2000 chars per message.
- No more than 10 messages per minute per token.

## Audit log

Every send attempt appends one JSON line to a log file on the host. Each line holds time, pane id, sender, text length, and result. The text itself is never logged. The file is append-only. Nothing in the bridge edits or deletes lines.

## Secrets

Output text is redacted before it leaves the bridge. Token-looking runs (long runs of token chars, same rule as the watch module) become `[redacted]`. Audit lines never hold message text. Preview shows the owner their own draft only.

## Failure answers

- Unknown pane id: 404, plain words.
- Pane outside the allowlist: 403, plain words.
- Bad or old confirm code: 403, plain words, with a fresh preview offered.
- Rate limit hit: 429 with a retry time.
- herdr CLI down: 503. The phone shows last-known output with its age.

## Phone screens

New page in the Hearth app (Agents):

1. List: each allowlisted pane with label, agent kind, and status dot. Read-only rows.
2. Pane view: latest output text, newest at the bottom. Shows the approval prompt banner when the pane waits on one.
3. Compose: text box plus Preview button. No send happens here.
4. Confirm sheet: the exact text, the target pane, the 60-second timer, Confirm and Cancel. Confirm sends once. Cancel drops the code.

No other screen sends text or keys. The sessions card on Builds stays read-only and unchanged.

## Tests (after approval)

- Fake herdr binary on PATH: canned pane list, output, and send recorder. No real panes in tests.
- Unit tests: pane id regex, confirm code expiry and single use, rate limit, redaction, allowlist checks, approval detection per kind table.
- API tests: each endpoint, each refusal above, audit line shape.
- UI tests: preview shows exact text, confirm sends once, cancel sends nothing.
- Live check: one allowlisted pane named by the owner, one short message, log inspected.

## Open questions for the owner

- Which pane is the live check target?
- Where should the config, token file, and audit log live on the host?
- Which agent kinds are allowed at first?
