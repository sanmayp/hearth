/** One API base value (build time). See web/README.md. */
const raw = (import.meta.env as Record<string, string | undefined>).PUBLIC_HEARTH_API_BASE;

/**
 * Where the app reads data.
 * - 'demo' (default): never fetch. Stash, then bundled samples.
 * - 'same': same origin (dev proxy, or served behind the API).
 * - URL: fetch that host.
 */
export const API_BASE = raw ?? 'demo';

/** True when the app must not attempt any fetch. */
export const DEMO_ONLY = API_BASE === 'demo';
