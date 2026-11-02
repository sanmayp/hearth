# Phone link (systemd user units)

The phone reaches the host through fixed loopback ports. Two user
units keep the link up across reboots. No secrets live here.

## Ports

- 8187: hearth-server API on 127.0.0.1.
- 4180: vite preview of the web build on 127.0.0.1.
- 8449: agent bridge on 127.0.0.1, fronted by the tailnet proxy.

## Secrets file (owner only)

Create `~/.config/hearth/server.env` with mode 600. It holds
three lines, plain `NAME=value`, no quotes, no export:

- DATABASE_URL
- REDIS_URL
- HEARTH_TOKEN

Values never get pasted anywhere else. The server unit reads
this file at start. It refuses to serve open when the token
is missing.

## Install

Build once first: `cargo build --release -p hearth-server -p agent-bridge`
from `~/hearth`, plus `npm run build` in `web/`.
Set up the bridge once too: `scripts/agent-bridge.sh setup`
plus `scripts/agent-bridge.sh allow <pane-id> <label>` per pane.

Link the units, reload, enable, start:

```
mkdir -p ~/.config/systemd/user
ln -sf ~/hearth/systemd/user/hearth-server.service ~/.config/systemd/user/
ln -sf ~/hearth/systemd/user/hearth-preview.service ~/.config/systemd/user/
ln -sf ~/hearth/systemd/user/hearth-bridge.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now hearth-server.service hearth-preview.service hearth-bridge.service
```

## Check

```
systemctl --user is-active hearth-server.service hearth-preview.service hearth-bridge.service
curl -sf http://127.0.0.1:8187/health/live
curl -sf http://127.0.0.1:4180/ -o /dev/null
curl -sf http://127.0.0.1:8449/health/live
```

## Owner wrappers (`~/bin`)

Short commands for Termius. No flags to type. Each prints
one clear line. Only `lo-key` prints a secret (the key alone).

Install once (one line, stays fresh from the repo):

```
mkdir -p ~/bin && ln -sf ~/hearth/scripts/local-bin/lo-key ~/hearth/scripts/local-bin/lo-restart ~/hearth/scripts/local-bin/lo-check ~/hearth/scripts/local-bin/lo-push ~/bin/
```

- `lo-key`: prints the bridge key for the Settings paste.
- `lo-restart`: restarts API, preview, and bridge, then checks all three.
- `lo-check`: runs the phone-link checks, prints one green or red line.
- `lo-push`: secret-scans the branch (paths only, never content), asks y/n once, then pushes.

## Change a port

Edit the unit (`PORT=`, `--port`), run `daemon-reload`,
then restart the unit. The app reads the same ports from
Settings, so update the phone too.
