# Hearth

Hearth is a control room for coding agents. It shows which agent windows really work. It shows which ones are stuck. It shows which ones wait for you.

Built with Muse (Meta) for the Meta Global AI Developer Hackathon.

![Hearth hero: a phone with a calm amber ring on dark glass](docs/images/hero.png)

## What it is

Hearth is one board for work and life. It lists coding agents across computers and projects. It holds tasks, reminders, and notes too.

## Why

One developer now runs many agents at once. They work on many projects and many computers. They fix bugs and add features at the same time. No one can track all that in memory. Agents give no board unless you read files or code. Hearth gives the board.

## Features

- Live list of agent windows across computers and projects.
- Truth check: working now, stuck, or waiting for you.
- Stuck reasons name billing and API errors.
- Needs-you list for approvals and questions.
- Approve or stop from the phone.
- Preview plus one-use code plus audit on every send.
- Voice commands from the phone, read-only by default.
- Tasks, reminders, and notes in the same place.

![Dev flow: agent lanes as a calm constellation](docs/images/dev-flow.png)

## How it works

The bridge reads terminal panes on each computer. It derives state from output signs. It never pastes screen text. The server merges panes into one feed. The phone app renders the board. Voice reads the board aloud. Writes need preview plus code. Details live in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

![Architecture: phone, private server, bridge, model endpoint](docs/images/architecture.png)

## Run it

Prereqs: Rust stable, Node 22. No keys needed for the demo.

```bash
git clone https://github.com/sanmayp/hearth.git
cd hearth
scripts/demo.sh
```

Full stack next: `make test`, then `cargo run -p hearth-server`. Then run `make ui-dev` for the phone UI.

## Built with Muse

Muse windows wrote the code. Claude reviewed it. Limits stay plain:

- Glasses speech is unverified on real hardware.
- Remote-host send stays OFF by default.
- AI batch runs in mock mode. No live calls ship in this repo.

## Safety

This repo holds no secrets. Voice is read-only by default. Writes need preview plus a one-use code. Every send lands in the audit log. Read [docs/SECURITY.md](docs/SECURITY.md) first.

## Status

Done: board, truth check, Needs-you list, approve and stop from the phone, voice commands, tasks and reminders.
Planned: Meta glasses audio on real hardware, live AI batch runs.

## License

MIT. See [LICENSE](LICENSE).
