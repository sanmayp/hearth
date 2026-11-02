---
id: REQ-001
title: Stack (decided)
status: proposed
owner_word: '2026-10-31 — "Backend: Rust. Cargo workspace. axum + tokio, sqlx, tracing, OpenTelemetry." (Part B §1)'
modules: [infra]
acceptance:
  - 'Backend is a Rust Cargo workspace using axum + tokio, sqlx, tracing, OpenTelemetry'
  - 'System of record is a dedicated `lifeos` PostgreSQL database on your own Kubernetes cluster, separate from other apps'
  - 'Redis is used for cache, Streams job queues, pub/sub live updates, rate limits, feature-flag cache, and short-term agent memory — and is never the only copy of anything'
  - 'Frontend is one SvelteKit + TypeScript codebase shipped as Tauri 2 desktop (Linux/Windows), Tauri 2 Android, and installable PWA (iPhone + any browser)'
  - 'No native iOS build until a Mac build host exists; PWA covers iPhone until then'
  - 'Live updates flow over WebSocket or SSE from Redis pub/sub'
  - 'Phone clients are offline-first: local cache plus sync queue'
  - 'Auth is passkeys (WebAuthn) plus device-bound sessions; single owner with room for family members later'
---

Source: Part B §1. Stack is decided, not up for re-debate.

## Notes

- One frontend codebase, three shippings. Platform-specific code stays
  behind adapters; no forks per platform.
- Redis holds derived/cached/transient state only. Anything that must
  survive a Redis flush lives in Postgres.
