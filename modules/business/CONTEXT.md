# Business module (CONTEXT.md)

Purpose: read-only view of shop data. KPIs,
bills, vendors, shifts, and notices.

Public API:
- `Business` owns `Module` (id "business").
- `GET /v1/business/overview` (KPIs, vendors, alerts).
- `GET /v1/business/invoices`, `GET /v1/business/schedule`.
- `Source` trait: `id` plus `fetch` (read-only pull).
- `SourceRegistry`: holds sources, merges pulls.
- `MemSource` is the test source.
- Store trait `BusinessStore`: `PgBusinessStore`, `MemBusinessStore`.
- Tools: `business.overview`, `business.invoices`, `business.schedule`.
- Widget: `business.today`. Job: `business.daily_brief`.
- Migration v1: table `business_cache` (key, data, updated_at).

Invariants:
- v1 is read-only. No POST, PUT, or DELETE exists.
- No store method writes shop data.
- Source down means last-known data plus stale time.

Gotchas:
- Owner connections land as later REQs. No live source ships now.
- Brief job is a def only. Worker follows.
- No finance advice text lives here. Facts only.
