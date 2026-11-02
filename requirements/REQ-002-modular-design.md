---
id: REQ-002
title: Modular design (Module trait)
status: proposed
owner_word: '2026-10-31 — "Each domain is a crate implementing one `Module` trait" (Part B §2)'
modules: [infra]
acceptance:
  - 'Each domain is its own crate implementing a `Module` trait with `id()`, `migrations()`, `routes()`, `jobs()`, `tools()`, `widgets()`, `health()`'
  - 'A module can be added, flagged on, or flagged off without redeploying the other modules'' data'
  - 'Modules communicate through typed events on Redis Streams, never through each other''s tables'
  - '`tools()` exposes only JSON-schema-described functions the agent may call; `widgets()` exposes dashboard cards the Today screen can render'
---

Source: Part B §2.

## Notes

- Cross-module reads go through events or each module's own API, never
  direct SQL into another module's tables. A test asserts no crate
  depends on another module's schema (sqlx query audit or deptree check).
