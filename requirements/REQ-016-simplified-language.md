---
id: REQ-016
title: Simplified language (ASD-STE100 style) for everything built
status: accepted
owner_word: '2026-10-31 — "Anything you build please use ASD-STE100 type language"'
modules: [ui, infra, agent]
acceptance:
  - 'All user-facing text (UI copy, errors, empty states, notifications, confirm taps, memory screen, crisis screen) uses short sentences (max ~20 words), one idea per sentence, common words, present tense, no idioms or jargon'
  - 'Warnings and safety text come before the action they guard (e.g. crisis screen, confirm-tap dialogs, delete flows)'
  - 'All project docs (REQ, CONTEXT.md, DECISIONS.md, user docs) follow the same rules'
  - 'Code comments are short, plain, and present-tense; no chain-of-thought in comments'
  - 'No marketing fluff that hides meaning; annotations and hints use plain words (extends REQ-013 chart annotations)'
  - 'Design-gate checklist (REQ-013) includes a plain-language check for every UI change'
---

Source: owner requirement, 2026-10-31.

## Notes

- "STE100 type" means adapted core rules, not certified spec
  compliance: short sentences, approved-simple vocabulary, one
  instruction per sentence, warnings first. No full-dictionary
  audit; reviewers check by reading.
- Code identifiers, API paths, and schema keys are excluded (they
  are code, not prose). Their docs and descriptions are included.
- This applies to helper-lane output too: briefs require it, and
  the controller checks it at merge.
