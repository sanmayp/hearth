# Architecture

Hearth splits into three parts. The app shows state. The server owns
state. The bridge reaches terminal agents. Each part runs alone.

![Architecture poster: phone, server, bridge, and model endpoint](images/architecture.png)

## Diagram

```mermaid
flowchart LR
    Phone[Phone app\nSvelteKit + Tauri] --> API[hearth-server\n/v1 API + auth]
    Phone --> Bridge[agent-bridge\npreview + confirm]
    Bridge --> Herd[Terminal agents\nherdr panes]
    API --> Mods[Modules\ntoday tasks health ...]
    Mods --> PG[(Postgres\nsystem of record)]
    API --> Cache[(Redis\ncache only)]
    Reader[AI batch\ntools/ai-batch] --> Models[Remote model\nOpenAI-compatible]
    API --> Reader
```

## Parts

- Web app: SvelteKit UI in `web/`. Tauri wraps it for phone and desktop.
- API server: `crates/hearth-server`. It mounts one domain crate per
  module under `/v1`. Bearer key guards all `/v1` routes.
- Modules: `modules/`. Each module owns its id, migrations, routes,
  jobs, tools, and widgets. Modules never read each other's tables.
- Agent bridge: `crates/agent-bridge`. It binds loopback only. It has
  its own token file. It sends text only after preview plus confirm.
- Batch: `tools/ai-batch`. It sends public text to a remote model
  or a local mock. Mock mode needs no key.
- Data: Postgres holds records. Redis holds cache and queues only.

## Request flow

1. The app calls `/v1/...` with a Bearer key.
2. The server checks the key, then routes to the module.
3. The module reads Postgres, or Redis for cache.
4. The app renders the answer in plain ASD-STE100 words.

## Bridge flow

1. The app lists allowlisted panes through the bridge.
2. The owner types a draft and taps Preview.
3. The app shows the exact text plus a 60-second timer.
4. The owner taps Confirm. The bridge checks code, allowlist,
   rate limit, kind, and status. Then it sends.
5. Each attempt appends one audit line. Text never logs.

See [BRIDGE](BRIDGE.md) for the full safeguard list.
