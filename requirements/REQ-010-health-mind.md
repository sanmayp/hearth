---
id: REQ-010
title: 'Module 7: Health & mind (with crisis safety)'
status: built
owner_word: '2026-10-31 — "Health & mind: mood and energy check-ins (10 seconds)" (Part B §4.7)'
modules: [health]
acceptance:
  - 'Mood and energy check-in completable in ~10 seconds'
  - 'Sleep, steps, heart data from Android Health Connect (HealthKit when the iOS app exists)'
  - 'Habits, journaling, breathing and grounding exercises, gentle CBT-style reflection prompts'
  - 'Weekly "what helped, what hurt" trends view'
  - 'SAFETY: no diagnoses and no medication advice anywhere in UI copy, prompts, or tools'
  - 'SAFETY: language suggesting crisis or self-harm immediately shows 988 (call/text, US) plus the owner''s chosen contact (owner-chosen contact, set at deploy), and stops the chat flow'
  - 'SAFETY: crisis detection runs as a local keyword/phrase check FIRST, working even when the model is off or unreachable'
  - 'All health and journal data counts as private data (REQ-003) and is encrypted at rest'
  - 'MVP scope is check-in only (see REQ-015); sensors, trends, and exercises follow'
---

Source: Part B §4.7. Build order: 7th.

## Notes

- Safety code path (crisis check, 988 surface, chat stop) requires a
  fresh reviewer lane "approve" plus owner confirmation before merge
  (Part A §1 rule 2). Reviewer brief must include the crisis-contact
  decision once the owner answers it.
