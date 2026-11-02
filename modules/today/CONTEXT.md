# Today module (CONTEXT.md)

Purpose: home dashboard data. First screen on all devices.
Rule picks one next action. No model call.

Public API:
- `Today` owns `Module` (id "today").
- `GET /v1/today` serves view plus next action.
- `PUT /v1/today` feeds snapshot (event worker).
- Store trait `TodayStore`: `PgTodayStore` prod, `MemTodayStore` tests.
- Tool `today.brief`. Widget `today.card`. Job `today.rollup`.
- Migration v1: table `today_snapshot` (one row).

Invariants:
- Empty snapshot serves with no action, never 500.
- PUT sets time to now on the server.
- Oldest due task wins, then reminder, then agenda.

Gotchas:
- Handlers are generic over the store. Register with turbofish.
- Tool defs are registry only. Run path lands with the agent core.
