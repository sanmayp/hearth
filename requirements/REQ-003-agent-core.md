---
id: REQ-003
title: Agent core (chat, voice, data guard, memory)
status: proposed
owner_word: '2026-10-31 — "Chat plus voice in, on every device. The model is Muse (Meta''s API) behind one `Llm` trait" (Part B §3)'
modules: [agent]
acceptance:
  - 'Chat plus voice input works on every device (desktop, Android, PWA)'
  - 'All model access goes through one `Llm` trait; provider is a config setting, and today it is Muse (Meta API)'
  - 'Every field sent toward the model carries a data class (general, private); private data (health, finance, journal, business records) reaches the model only for modules the owner switched on for AI in Settings'
  - 'AI is OFF by default for every module; each module is enabled individually in Settings (owner decision 2026-10-31)'
  - 'Modules not enabled for AI still work, using plain rules with no model call'
  - 'Each model call sends the fewest fields that answer the question, never whole records'
  - 'Tool calling targets module `tools()` only; writes that make payments, send messages, or delete data require a confirm tap'
  - 'Long-term memory is rows in Postgres (with source and date) plus pgvector embeddings; short-term context is in Redis'
  - 'Owner can view, edit, and delete any memory from a screen'
  - 'Prompt caching on the stable system prefix, summaries instead of raw history, and a per-request token budget visible in a dev panel'
---

Source: Part B §3.

## Notes

- The data guard runs before EVERY model call, no exceptions. A test
  feeds private-class fields with AI disabled for that module and
  asserts no model call is made.
- Confirm-tap rule: the agent proposes, the owner taps. No silent
  spend/send/delete path exists.
