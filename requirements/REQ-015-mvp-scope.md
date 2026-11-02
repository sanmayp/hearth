---
id: REQ-015
title: MVP scope (first shippable)
status: accepted
owner_word: '2026-10-31 — "Today screen + Reminders + Tasks + Inventory (manual entry and scan) + Health check-in" (Part B §7)'
modules: [today, reminders, tasks, inventory, health, infra, agent, ui]
acceptance:
  - 'MVP ships: Today screen (REQ-004) + Reminders (REQ-005) + Tasks (REQ-006) + Inventory manual entry and scan (REQ-007 subset) + Health check-in (REQ-010 subset)'
  - 'Runs on desktop (Tauri), Android (Tauri), and iPhone (PWA)'
  - 'Deployed on your own Kubernetes cluster through the Part A pipeline (gates G0/G1/G2, feature flags, canary, `make ship`)'
  - 'Each MVP module ships dark behind a flag and is switched on from Settings'
  - 'Everything else (REQ-008, REQ-010 remainder, REQ-011, REQ-012) follows one module per train, in Part B §4 order'
---

Source: Part B §7. Status stays `proposed` until the owner approves
the MVP cut; then it becomes `accepted` and the build starts.
