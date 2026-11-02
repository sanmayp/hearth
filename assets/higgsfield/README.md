# Higgsfield art

Generated 2026-10-31 with the Higgsfield MCP. One style: warm 3D illustration, ink-navy (#0E1320), one amber ember glow, no text.

Rules:
- Product art only. No dev diagrams or images here.
- Each file stays under 300 KB. Set width and height in the markup to stop layout shift.
- No runtime calls to Higgsfield. The app serves these files itself.
- Originals (PNG, raw MP4) sit in `src/`. That folder is git-ignored. Re-compress from it if needed.

## Stills (WebP, quality 80)

| File | Size (px) | Use |
|---|---|---|
| `avatar-ember-still.webp` | 512x512 | Avatar poster frame, reduced-motion fallback |
| `module-reminders.webp` | 1536x1020 | Reminders header |
| `module-tasks.webp` | 1536x1020 | Tasks header |
| `module-items.webp` | 1536x1020 | Items header (busier than the rest; crop if needed) |
| `module-health.webp` | 1536x1020 | Health header |
| `module-business.webp` | 1536x1020 | Business header |
| `empty-state.webp` | 768x768 | Empty lists |
| `offline-state.webp` | 768x768 | Offline banner or page |
| `error-state.webp` | 768x768 | Error page |
| `bg-ambient-still.webp` | 1920x1086 | Today background, reduced-motion fallback |
| `module-builds.webp` | 1024x688 | Builds header |
| `module-agents.webp` | 1024x688 | Agents strip |

## Loops (H.264, no audio, faststart; first frame = last frame)

| File | Size (px), length | Use |
|---|---|---|
| `avatar-ember-idle.mp4` | 960x960, 4 s | Avatar at rest |
| `avatar-ember-listen.mp4` | 960x960, 3 s | Avatar while listening (spec said 2 s; model minimum is 3 s) |
| `avatar-ember-think.mp4` | 960x960, 3 s | Avatar while thinking |
| `bg-ambient-today.mp4` | 1276x720, 10 s | Today background |

Play loops with `autoplay muted loop playsinline`. Under `prefers-reduced-motion`, show the matching still.

## Spend

Credits: 644.5 before, 610.75 after = 33.75 total. About 25 went to the four Kling 3.0 loops; 8.75 went to the 11 stills (GPT Image 2.5). Later: 2 more stills (builds, agents) at 0.5 each = 1.0.
