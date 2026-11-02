/** Thin update glue. Core checks live in the Rust shell. No-ops outside Tauri. */
import { API_BASE } from './config';
import { loadToken } from './api';

export type UpdateState = {
  status: 'unknown' | 'current' | 'available' | 'error';
  version: string;
  notes: string;
  detail: string;
  at: string;
};

const KEY = 'hearth:update';

/** True only inside the Tauri shell. */
function tauri(): boolean {
  try {
    return '__TAURI_INTERNALS__' in globalThis;
  } catch {
    return false;
  }
}

/** Last update state, for any page to read. */
export function loadUpdateState(): UpdateState {
  try {
    const raw = globalThis.localStorage?.getItem(KEY);
    if (raw) return JSON.parse(raw) as UpdateState;
  } catch {
    /* fall through */
  }
  return { status: 'unknown', version: '', notes: '', detail: '', at: '' };
}

function saveUpdateState(s: UpdateState): void {
  try {
    globalThis.localStorage?.setItem(KEY, JSON.stringify(s));
  } catch {
    /* private mode */
  }
}

type CheckOut = { available: boolean; version: string; notes: string; code: number };

/** One silent check. Returns the state and keeps it for Settings. */
export async function runUpdateCheck(): Promise<UpdateState> {
  const now = new Date().toISOString();
  if (!tauri()) {
    return { status: 'unknown', version: '', notes: '', detail: 'Update check needs the Android app.', at: now };
  }
  const token = loadToken();
  if (!token) {
    const s: UpdateState = {
      status: 'unknown',
      version: '',
      notes: '',
      detail: 'Paste the server key first.',
      at: now
    };
    saveUpdateState(s);
    return s;
  }
  if (API_BASE === 'demo' || API_BASE === 'same') {
    const s: UpdateState = {
      status: 'unknown',
      version: '',
      notes: '',
      detail: 'This build cannot reach the update server.',
      at: now
    };
    saveUpdateState(s);
    return s;
  }
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const out = (await invoke<CheckOut>('check_update', { apiBase: API_BASE, bearer: token })) as CheckOut;
    const s: UpdateState = out.available
      ? { status: 'available', version: out.version, notes: out.notes, detail: '', at: now }
      : { status: 'current', version: out.version, notes: '', detail: '', at: now };
    saveUpdateState(s);
    return s;
  } catch (e) {
    const cause = checkFailureDetail(e);
    try {
      console.warn('[update] check failed:', cause);
    } catch {
      /* no console */
    }
    const s: UpdateState = { status: 'error', version: '', notes: '', detail: cause, at: now };
    saveUpdateState(s);
    return s;
  }
}

/**
 * Plain cause for a failed check. The Rust core already speaks
 * plain words, so its text ships as-is. Anything else falls back
 * to the link line. Pure, tested.
 */
export function checkFailureDetail(e: unknown): string {
  if (typeof e === 'string' && e.trim()) return e.trim().slice(0, 200);
  if (e instanceof Error && e.message) return e.message.slice(0, 200);
  return 'Update check failed. Check the link and retry.';
}

/** Download, verify, and open the system installer. Shows plain progress. */
export async function runUpdateInstall(onStep: (msg: string) => void): Promise<string> {
  const token = loadToken();
  const { invoke } = await import('@tauri-apps/api/core');
  onStep('Fetching the new build.');
  await invoke('download_update', { apiBase: API_BASE, bearer: token });
  onStep('Verified. Opening the installer.');
  await invoke('install_update');
  return 'opening installer. Tap Install once.';
}
