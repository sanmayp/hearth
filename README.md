# Hearth

Hearth is a calm center for life, health, and AI agents. It runs on your own hardware. It speaks in short plain sentences. It never sends a secret anywhere.

Built with Muse (Meta) for the Meta Global AI Developer Hackathon.

![Hearth hero: a phone with a calm amber ring on dark glass](docs/images/hero.png)

## Built with

- Muse (Meta): the developer of this repo.
- Built for the Meta Global AI Developer Hackathon.

## What it does

- Home: one ring shows the day. One card names the next step.
- Dev flow: watch coding agents across panes, lanes, and commits.
- Needs You: approvals and questions surface in one list.
- Agent bridge: read and message terminal agents from the phone. Preview plus confirm. Full audit.
- Feature tracker: every feature carries status, proof, and links.
- Self-update: signed over-the-air app updates with rollback.
- Plain text: all UI copy follows ASD-STE100 rules. See [docs/ASD-STE100.md](docs/ASD-STE100.md).
- Meta glasses companion: planned. Glanceable cards, voice-first answers.

![Dev flow: agent lanes as a calm constellation](docs/images/dev-flow.png)

## AI batch (mock mode)

Hearth reads public text through `tools/ai-batch`. Mock mode runs
with no key and is what this submission demonstrates. No live calls
are recorded in this repo.

![Architecture: phone, private server, bridge, model endpoint](docs/images/architecture.png)

## 5-minute quickstart

Prereqs: Rust stable, Node 22. No keys needed for the demo.

```bash
git clone https://github.com/sanmayp/hearth.git
cd hearth
scripts/demo.sh
```

The demo prints the route manifest, shows the bridge refusing to start
without its token file, and runs the AI batch in mock mode.

Full stack next: set `DATABASE_URL`, `REDIS_URL`, and `HEARTH_TOKEN`,
then `cargo run -p hearth-server`. Point the web app at it with
`HEARTH_API=http://127.0.0.1:8080` and `npm run dev` in `web/`.

## AI usage (metered, honest)

`tools/ai-batch` fans public text out to a remote model or a local
mock, with concurrency caps, cost caps, retries, and per-call
meters. Mock runs stay labelled mock and never mix with live
numbers.

- Mock batch (no key, no network): `scripts/demo.sh` runs it as
  step 3, or `cargo run -q -p ai-batch -- run --provider=mock`.
- Remote batch: set `HEARTH_AI_URL`, `HEARTH_AI_KEY`, and
  `HEARTH_AI_MODEL`, then
  `cargo run -q -p ai-batch -- run --provider=remote --max-calls=200`.
- Dry-run first (no calls, no spend): add `--dry-run`.

The last report (4 mock calls, 0 live) lives in
[docs/USAGE-REPORT.md](docs/USAGE-REPORT.md), generated from real
meter files, never hand-written. The app shows the same numbers
on Settings and Dev flow.

## Repo map

- `crates/hearth-server`: API server. All modules mount under `/v1`.
- `crates/agent-bridge`: phone-to-terminal bridge. Loopback only.
- `modules/`: one domain per crate. Each owns its routes and migrations.
- `web/`: SvelteKit app plus Tauri shell for phone and desktop.
- `tools/ai-batch`: batch runner. Mock mode needs no key.
- `docs/`: architecture, bridge, dev flow, security.

## Docs

- [ARCHITECTURE](docs/ARCHITECTURE.md) — parts and data flow.
- [BRIDGE](docs/BRIDGE.md) — agent bridge safeguards.
- [DEV-FLOW](docs/DEV-FLOW.md) — how lanes ship.
- [SECURITY](docs/SECURITY.md) — keys and safe setup.
- [ASD-STE100](docs/ASD-STE100.md) — the writing standard.

## Security

This repo holds no secrets. Set your own keys at deploy.
Read [docs/SECURITY.md](docs/SECURITY.md) first.

## License

MIT. See [LICENSE](LICENSE).
