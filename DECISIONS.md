# DECISIONS.md (append-only — never rewrite history)

## 2026-10-31 — iPhone ships as PWA for MVP

Context: MVP must run on iPhone; native iOS needs a Mac build host we don't have.
Decision: Ship iPhone as installable PWA (Web Push for reminders); revisit native iOS after MVP.
Reason: Zero extra cost, ships with MVP, no new build infra; PWA covers all MVP needs on iPhone.
Owner: "PWA for now (Recommended)" (Q1 answer, 2026-10-31).
Affects: REQ-001 (stack), REQ-005 (reminders push), REQ-015 (MVP scope).

## 2026-10-31 — Agent name: Hearth

Context: "LifeOS" was a working name; owner asked for 3 candidates and picked one.
Decision: The agent is named Hearth. Single source of truth: `requirements/00-name.md`.
Reason: Warm center-of-home concept; matches the dark-first + one-warm-accent design direction; short, memorable, obvious avatar concept (ember/glow).
Owner: picked "1 (Hearth)" from controller's 3-name pitch (Q2 answer, 2026-10-31).
Affects: 00-name.md, REQ-013 (visual design, avatar).

## 2026-10-31 — Avatar style: warm ember orb

Context: Higgsfield brief needs an avatar direction for Hearth.
Decision: Abstract warm ember orb on dark; still + idle-breathe, listen-pulse, think-shimmer loops.
Reason: Embodies the name, stays classy at the award-level bar, and the three animation states map naturally onto a glow.
Owner: picked "1" (Q3 answer, 2026-10-31).
Affects: REQ-013 (visual design, assets).

## 2026-10-31 — AI default-off for ALL modules, per-module opt-in

Context: Owner decides what leaves owner hardware for Muse (Meta servers).
Decision: AI is off for every module by default; each module is switched on individually from Settings. All modules work AI-free via rules until enabled.
Reason: Strongest privacy default; owner opts in per module once value is proven. Stricter than the Q4 recommendation (which was private-modules-off only).
Owner: "AI off for everything for now but can be turn on by modules" (Q4 answer, 2026-10-31).
Affects: REQ-003 (agent core, data guard), REQ-010 (health), all module REQs.

## 2026-10-31 — Crisis contact: owner-chosen contact plus 988

Context: Health module shows 988 plus the owner's chosen contact on crisis language.
Decision: Chosen contact plus 988. The contact ships as a placeholder; the owner sets the real one at deploy. Stored in REQ-010; shown alongside 988 (call/text, US), chat flow stops.
Reason: Owner-provided; a known person next to 988 is the point of the feature.
Owner: contact kept private; placeholder ships in the public copy (Q5 answer, 2026-10-31).
Affects: REQ-010 (health & mind).

## 2026-10-31 — Higgsfield via CLI, limited calls

Context: Owner wants automated asset generation with limited Higgsfield spend.
Decision: Use `@higgsfield/cli` (`higgsfield auth login` once by owner, then controller runs `generate create/cost`, `account` for balance). Verify cost with `generate cost` before each batch; check `account` balance before and after. No fixed dollar cap; keep calls minimal per budget.md.
Reason: CLI is Higgsfield's coding-agent path (low overhead, same plan credits as MCP, no API key); `cost` + `account` give exact spend control.
Owner: followed the CLI setup (2026-10-31); "don't burn too much higgsfield token"; "$140 is your muse cap, higgsfield calls only limited".
Affects: REQ-013 (visual design, assets).

## 2026-10-31 — API auth: bearer token on /v1 (Draft)

Context: Spec sweep found the API open (no auth). Owner picked the auth track first for tonight.
Decision: Bearer token from env (`HEARTH_TOKEN`), checked on every `/v1` route. `/health/live` and `/health/ready` stay open for probes. Fail closed: server refuses to start without a token.
Reason: Simplest shape that works on tailnet and phone tonight; matches the env-file habit. Passkeys stay the long-term ask.
Owner: "Lock down API auth", "Bearer token", "/v1 locked /health open" (interactive answers, 2026-10-31).
Affects: hearth-server main.rs, G2 script, PWA api.ts.

## 2026-10-31 — PWA token entry: Settings page (Draft)

Context: Phone app needs the bearer token without baking it into the build.
Decision: A Settings page holds one token field; the key stays in device localStorage and rides on every call. Never baked into the bundle.
Reason: One paste per device, no secret in git or builds.
Owner: "Settings page" (interactive answer, 2026-10-31).
Affects: web routes settings, api.ts, layout nav.
