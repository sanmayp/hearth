# Hearth web app (SvelteKit, static)

One codebase ships as Tauri desktop, Tauri Android, and PWA.

## API base URL (one value)

`PUBLIC_HEARTH_API_BASE` (build time, `src/lib/config.ts`):

- unset or `demo` (default): never fetch. Screens show the
  last-known data when present, else bundled samples.
- `same`: same origin. Use with the dev proxy or when served
  behind the API. (`npm run dev` sets this.)
- full URL: fetch that host. Example:
  `PUBLIC_HEARTH_API_BASE=http://127.0.0.1:8081 npm run build`.

Preview builds default to demo mode, so `npm run preview`
shows full screens with zero API calls and zero console
fetch errors. For live preview, build with the URL first,
then preview. Always rebuild before preview: hashed asset
names change per build, and a stale server shows old files.

## Offline rule

Each read stashes its last good answer (localStorage, memory
fallback). On failure the screen shows the stash with its age
("5 min ago", "stale since …"), else samples. No screen stays
blank. No fetch means no console error.

## Checks

- `npm run check`: types. `npm test`: unit tests.
- `npm run e2e`: Playwright. Builds first, serves preview,
  checks the styled shell, demo mode with zero failed
  requests, and writes shots to `e2e/shots/`.
