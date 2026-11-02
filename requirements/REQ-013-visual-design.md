---
id: REQ-013
title: Visual design (award-level bar)
status: proposed
owner_word: '2026-10-31 — "the bar: award-winning, Apple Design Award / Awwwards ''Site of the Day'' level" (Part B §5)'
modules: [ui]
acceptance:
  - 'One idea: "a calm cockpit for your life" — dark-first deep ink background, one warm accent, generous space; every screen answers "what needs me now?" in under two seconds'
  - 'Signature moment 1: Today opens with a radial "life wheel" that blooms in (staggered, 400–600 ms, spring easing) then settles into cards; each segment is a module, fill is today''s state'
  - 'Signature moment 2: assistant avatar is alive — idle breathing loop, pulse while voice is on, shimmer "think" state, never a spinner'
  - 'Signature moment 3: inventory floor plan in isometric 3D with smooth pan/pinch; tapping a room flies the camera in and lists contents'
  - 'Signature moment 4: health trends as hand-tuned charts — soft area gradients, plain-word annotations, scrub gesture with haptics on phone'
  - 'Motion system: durations 120/240/400 ms and two easings (standard, spring) in tokens; shared-element card→detail transitions; every animation has a reduced-motion crossfade equivalent'
  - 'Type: one variable sans + one display face for big numbers; strict 6-step scale; tabular figures for every number; one icon set, one stroke weight'
  - 'Colour tokens in one file (light + dark); one hue per module used sparingly; 4.5:1 text and 3:1 UI-shape contrast in both themes'
  - '60 fps on mid-range Android (tested on a real device); first screen interactive < 1.5 s on 4G; no layout shift; skeletons shaped like content (no spinners)'
  - 'Empty, error, and offline states are designed screens with art + one clear next action'
  - 'One-thumb phone use (primary actions in bottom third); full keyboard desktop use with Ctrl/Cmd-K palette'
  - 'Micro-interactions: haptic tick on complete, check-off animation, pull-to-refresh with avatar, undo toast on every destructive action'
  - 'Art from higgsfield.ai (avatar still + idle/listen/think loops, module illustrations, empty-state art, ambient loops) in one style, stored in `assets/`, compressed (AVIF/WebP, small MP4/Lottie, under 300 KB each); no runtime dependency on Higgsfield'
  - 'Design gate (part of G1 for any UI change): Playwright screenshots at 390 px + 1440 px, both themes, to the design lane with the 10-point checklist; below 8/10 goes back; owner sees screenshots before a module ships'
---

Source: Part B §5.
