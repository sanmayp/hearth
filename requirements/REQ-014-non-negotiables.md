---
id: REQ-014
title: Non-negotiables (privacy, resilience, read-only v1, a11y)
status: proposed
owner_word: '2026-10-31 — "Privacy: owner data never leaves owner-controlled infra, except in model calls the data guard allows." (Part B §6)'
modules: [infra, agent, ui]
acceptance:
  - 'Owner data never leaves owner-controlled infra except model calls the data guard allows (REQ-003)'
  - 'No analytics SDKs anywhere in any client'
  - 'Full export (JSON plus files) and full delete both work end to end'
  - 'Every module works with its data source down: shows last-known data with a "stale since" label'
  - 'v1 is read-only for all business integrations (REQ-008)'
  - 'Accessibility: screen-reader labels, 4.5:1 contrast, text scaling'
---

Source: Part B §6. Applies to every module and every train.
