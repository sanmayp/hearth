# Tasks module (CONTEXT.md)

Purpose: inbox, today, someday. Rule splitter cuts
big tasks. Undo brings back done tasks.

Public API:
- `Tasks` owns `Module` (id "tasks").
- `GET /v1/tasks[?list=]`, `POST /v1/tasks`.
- `POST /v1/tasks/:id/done`, `/undo`, `/move`, `/split`.
- `POST /v1/tasks/:id/steps/:n/done`.
- Store trait `TaskStore`: `PgTaskStore`, `MemTaskStore`.
- Rule `split_rule` (" then ", " and ", "; ").
- Migration v1: table `tasks` (steps JSONB).

Invariants:
- List hides done tasks. Split runs once only.
- Unknown list names land in inbox.

Gotchas:
- Agent split refines steps when AI turns on.
- Rollover job is a def only. Worker follows.
