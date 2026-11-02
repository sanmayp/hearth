# Hello module (CONTEXT.md)

Purpose: prove the factory path end to end.
It owns one table, one route, one tool, one card.

Public API:
- `Hello` owns `Module` (id "hello").
- `GET /v1/hello` answers `{"ok":true,...}`.
- Tool `hello.say` (test only). Widget `hello.card`.
- Migration v1: table `hello_ping`.

Invariants:
- No writes except migration. No private data.
- Health runs `SELECT 1` on the module pool.

Gotchas:
- Routes take `AppState` but use no state yet.
- Keep this file under 60 lines. Read code, not dirs.
