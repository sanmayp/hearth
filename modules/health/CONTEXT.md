# Health module (CONTEXT.md)

Purpose: 10-second check-in. Crisis guard runs local
and first. All payloads seal with AES-256-GCM.

Public API:
- `HealthMod` owns `Module` (id "health").
- `POST /v1/health/checkin` (validates, crisis-checks, seals).
- `GET /v1/health/checkins[?days=]`, `GET /v1/health/status`.
- Store trait `HealthStore`: `PgHealthStore`, `MemHealthStore`.
- `crisis_hit`, `seal`, `open`, `key_from_env`.
- Key: `HEALTH_KEY` env (base64, 32 bytes). No key: dark.
- Migration v1: table `health_checkin` (sealed payload).

Safety (merge blockers hold):
- No diagnoses. No drug advice. Ever.
- Crisis words show 988 plus the owner-chosen contact (set at deploy).
- Client stops the chat flow on `crisis: true`.
- Health counts as private data (data guard).

Gotchas:
- Zero key `[0u8; 32]` fallback never serves prod:
  health() reports down, flag stays off.
- Trends, sensors, exercises land after MVP.
