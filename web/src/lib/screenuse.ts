/** Screen open tracking. First and last open per route, on device. */
import { apiGet, apiPost, loadToken } from './api';

export type ScreenUse = { first: string; last: string };
export type ScreenUseMap = Record<string, ScreenUse>;
export type ScreenSyncMap = Record<string, string>;
export type ScreenConfirm = { confirmed: string; confirmed_at: string };

const USE_KEY = 'hearth:screen-use';
const SYNC_KEY = 'hearth:screen-sync';
/** Sync a route at most once an hour. Opens stay local otherwise. */
export const SYNC_EVERY_MS = 60 * 60 * 1000;

function storage(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

function loadRecords(key: string): Record<string, { first?: unknown; last?: unknown }> {
  const store = storage();
  if (!store) return {};
  try {
    const raw = store.getItem(key);
    if (!raw) return {};
    const val: unknown = JSON.parse(raw);
    if (!val || typeof val !== 'object' || Array.isArray(val)) return {};
    return val as Record<string, { first?: unknown; last?: unknown }>;
  } catch {
    return {};
  }
}

/** Device open records. Junk storage reads as empty, never throws. */
export function loadUse(): ScreenUseMap {
  const out: ScreenUseMap = {};
  for (const [route, rec] of Object.entries(loadRecords(USE_KEY))) {
    if (typeof rec?.first === 'string' && typeof rec?.last === 'string') {
      out[route] = { first: rec.first, last: rec.last };
    }
  }
  return out;
}

/** Last sync stamps per route. Junk reads as empty. */
export function loadSync(): ScreenSyncMap {
  const store = storage();
  if (!store) return {};
  try {
    const val: unknown = JSON.parse(store.getItem(SYNC_KEY) ?? '');
    if (!val || typeof val !== 'object' || Array.isArray(val)) return {};
    const out: ScreenSyncMap = {};
    for (const [route, at] of Object.entries(val as Record<string, unknown>)) {
      if (typeof at === 'string') out[route] = at;
    }
    return out;
  } catch {
    return {};
  }
}

function stamp(now: Date): string {
  const p = (n: number): string => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())} ${p(now.getHours())}:${p(now.getMinutes())}`;
}

/** Record one open. First stays earliest, last moves forward. */
export function recordOpen(route: string, now = new Date()): ScreenUseMap {
  const map = loadUse();
  const at = stamp(now);
  const prev = map[route];
  map[route] = { first: prev?.first ?? at, last: at };
  storage()?.setItem(USE_KEY, JSON.stringify(map));
  return map;
}

/** True after one open. A screen counts as used, nothing more. */
export function isUsed(map: ScreenUseMap, route: string): boolean {
  return map[route] !== undefined;
}

/** Split known screens into unopened and opened. Order kept. */
export function splitScreens(
  routes: string[],
  map: ScreenUseMap
): { unopened: string[]; done: string[] } {
  const unopened: string[] = [];
  const done: string[] = [];
  for (const r of routes) {
    if (isUsed(map, r)) done.push(r);
    else unopened.push(r);
  }
  return { unopened, done };
}

/** True when the route never synced or the hour passed. */
export function dueForSync(sync: ScreenSyncMap, route: string, now = new Date()): boolean {
  const at = sync[route];
  if (!at) return true;
  const then = new Date(at.replace(' ', 'T')).getTime();
  if (Number.isNaN(then)) return true;
  return now.getTime() - then >= SYNC_EVERY_MS;
}

/** Stamp a route synced now. */
export function markSynced(route: string, now = new Date()): void {
  const sync = loadSync();
  sync[route] = stamp(now);
  storage()?.setItem(SYNC_KEY, JSON.stringify(sync));
}

/** Sync one open record to the ledger. False when quiet, never throws. */
export async function syncOpen(route: string, first: string, last: string): Promise<boolean> {
  if (!loadToken()) return false;
  try {
    await apiPost('/watch/screens/open', { route, first_open: first, last_open: last });
    return true;
  } catch {
    return false;
  }
}

/** Post a Works/Broken verdict for a screen. */
export async function confirmScreen(
  route: string,
  confirmed: 'works' | 'broken'
): Promise<{ ok: boolean; confirmed_at: string; error: string }> {
  try {
    const res = await apiPost<{ confirmed_at?: string }>('/watch/screens/confirm', {
      route,
      confirmed
    });
    return { ok: true, confirmed_at: res.confirmed_at ?? '', error: '' };
  } catch (e) {
    const msg =
      (e as { message?: string })?.message ?? 'Verdict is unsent. Retry soon.';
    return { ok: false, confirmed_at: '', error: msg };
  }
}

/** Read confirmed verdicts from the ledger. Missing means none. */
export async function loadConfirms(): Promise<Record<string, ScreenConfirm>> {
  const out: Record<string, ScreenConfirm> = {};
  try {
    const res = await apiGet<{ screens?: Record<string, ScreenConfirm> }>('/watch/screens');
    for (const [route, row] of Object.entries(res.data.screens ?? {})) {
      if (row && (row.confirmed === 'works' || row.confirmed === 'broken')) {
        out[route] = { confirmed: row.confirmed, confirmed_at: row.confirmed_at ?? '' };
      }
    }
  } catch {
    // Quiet: the checklist works from device records alone.
  }
  return out;
}
