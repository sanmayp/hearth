---
id: REQ-008
title: 'Module 5: Business (read-only adapter)'
status: built
owner_word: '2026-10-31 — "Business: a generic adapter interface (connect any system by API or DB view) that is read-only in v1." (Part B §4.5)'
modules: [business]
acceptance:
  - 'Generic adapter interface: connect any system by API or DB view'
  - 'v1 is strictly read-only — no write path exists in code (see REQ-014)'
  - 'Covers KPIs, invoices and vendors, staff schedule, and alerts'
  - 'Daily business brief is generated'
  - 'NOT in MVP; owner adds specific connections later as their own REQ files'
---

Source: Part B §4.5. Build order: 5th (post-MVP).
