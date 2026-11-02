# Reminders module (CONTEXT.md)

Purpose: one-shot and repeat reminders, snooze, due list.
Push plus calendar sync follow (need live servers).

Public API:
- `Reminders` owns `Module` (id "reminders").
- `GET /v1/reminders[?due_only=1]`, `POST /v1/reminders`.
- `POST /v1/reminders/:id/done`, `POST /v1/reminders/:id/snooze`.
- Store trait `ReminderStore`: `PgReminderStore`, `MemReminderStore`.
- Rules: `next_due` (DAILY, WEEKLY, INTERVAL, UNTIL), `is_due`.
- Jobs `reminders.dispatch`, `reminders.cal_sync` (defs only now).
- Migration v1: table `reminders`.

Invariants:
- List hides done items. Snooze hides until its time.
- Unknown RRULE falls back to the plain time.
- Done one-shots never fire again.

Gotchas:
- Full RFC RRULE lands with the dispatch worker.
- Push needs ntfy plus Web Push keys (owner servers).
