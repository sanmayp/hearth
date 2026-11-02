/** Small API client. Plain errors, stash, demo default. */
import { API_BASE, DEMO_ONLY } from './config';

export type ApiError = { message: string; status: number };

type Stash = { data: unknown; at: string };
const mem = new Map<string, Stash>();

/** Load last-known data for a path (localStorage, else memory). */
export function loadStash<T>(path: string): { data: T; at: Date } | null {
  try {
    const raw = globalThis.localStorage?.getItem('hearth:' + path);
    if (raw) {
      const s = JSON.parse(raw) as Stash;
      return { data: s.data as T, at: new Date(s.at) };
    }
  } catch {
    /* fall through to memory */
  }
  const s = mem.get(path);
  return s ? { data: s.data as T, at: new Date(s.at) } : null;
}

/** Save last-known data for a path. */
export function saveStash(path: string, data: unknown): void {
  const s: Stash = { data, at: new Date().toISOString() };
  mem.set(path, s);
  try {
    globalThis.localStorage?.setItem('hearth:' + path, JSON.stringify(s));
  } catch {
    /* private mode: memory only */
  }
}

/** Base for direct calls. Demo and same map to same-origin shape. */
function base(): string {
  return API_BASE === 'demo' || API_BASE === 'same' ? '' : API_BASE;
}

/** Bearer token from device storage. Empty when the owner never pasted one. */
export function loadToken(): string {
  try {
    return globalThis.localStorage?.getItem('hearth:token') ?? '';
  } catch {
    return '';
  }
}

/** Keep the token on this device only. Never send it anywhere but Hearth. */
export function saveToken(v: string): void {
  try {
    globalThis.localStorage?.setItem('hearth:token', v);
  } catch {
    /* private mode: no store */
  }
}

function authHeaders(extra: Record<string, string> = {}): Record<string, string> {
  const t = loadToken();
  return t ? { ...extra, Authorization: `Bearer ${t}` } : extra;
}

/** Plain error for a refused call. A 401 means the key is missing or wrong. */
function refused(status: number): ApiError {
  if (status === 401) {
    return { message: 'Hearth needs your key. Open Settings and paste it.', status };
  }
  return { message: 'Hearth said no. Retry soon.', status };
}

/** One read with last-known fallback. Returns data plus fetch time. */
export async function apiGet<T>(path: string): Promise<{ data: T; at: Date }> {
  let res: Response;
  try {
    res = await fetch(`${base()}/v1${path}`, { headers: authHeaders() });
  } catch {
    throw { message: 'No link to Hearth. Check your link and retry.', status: 0 } as ApiError;
  }
  if (!res.ok) {
    throw refused(res.status);
  }
  const data = (await res.json()) as T;
  const out = { data, at: new Date() };
  saveStash(path, data);
  return out;
}

/** One write. Plain errors like reads. */
export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${base()}/v1${path}`, {
      method: 'POST',
      headers: authHeaders({ 'Content-Type': 'application/json' }),
      body: JSON.stringify(body)
    });
  } catch {
    throw { message: 'No link to Hearth. Check your link and retry.', status: 0 } as ApiError;
  }
  if (!res.ok) {
    throw refused(res.status);
  }
  return (await res.json()) as T;
}

/**
 * Read with a local fallback. `live` tells if the server answered,
 * `stale` tells if the data is last-known (not fresh, not sample).
 * Demo mode never fetches, so it never logs console errors.
 */
export async function apiGetOr<T>(
  path: string,
  fallback: T
): Promise<{ data: T; at: Date; live: boolean; stale: boolean }> {
  if (!DEMO_ONLY) {
    try {
      const res = await apiGet<T>(path);
      return { ...res, live: true, stale: false };
    } catch {
      /* fall through to stash, then samples */
    }
  }
  const s = loadStash<T>(path);
  if (s) return { data: s.data, at: s.at, live: false, stale: true };
  return { data: fallback, at: new Date(), live: false, stale: false };
}

/** Plain age label: "just now", "5 min ago", or full stale note. */
export function ageLabel(at: Date, now = new Date()): string {
  const mins = Math.floor((now.getTime() - at.getTime()) / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} h ago`;
  return `stale since ${at.toLocaleDateString()}`;
}
