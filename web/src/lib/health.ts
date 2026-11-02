/** Link health checks for the Needs You strip. Plain reasons only. */
import { loadToken } from './api';
import { bridgeBase, bridgeGet, loadBridgeKey, type BridgeError } from './bridge';
import { runUpdateCheck } from './update';

export type HealthState = { ok: boolean; why: string };

/** Server key check. It reads presence only, never the value. */
export function keyHealth(token: string): HealthState {
  return token
    ? { ok: true, why: 'Key kept.' }
    : { ok: false, why: 'No server key. Open Settings.' };
}

/** Data link check from a feed load result. */
export function dataHealth(live: boolean, error: string): HealthState {
  return live ? { ok: true, why: 'Live.' } : { ok: false, why: error || 'No link.' };
}

/** Update manifest check from a stored update state. */
export function updateHealth(s: {
  status: string;
  version: string;
  detail: string;
}): HealthState {
  if (s.status === 'current') return { ok: true, why: 'App is current.' };
  if (s.status === 'available')
    return { ok: true, why: s.version ? `Update ${s.version} ready.` : 'Update ready.' };
  return { ok: false, why: s.detail || 'Update check failed.' };
}

/** Probe the bridge panes endpoint. No key or link means red, never a call. */
export async function checkBridge(): Promise<HealthState> {
  if (!loadBridgeKey() || !bridgeBase()) {
    return { ok: false, why: 'Add the bridge key in Settings.' };
  }
  try {
    await bridgeGet<{ panes: unknown[] }>('/v1/panes');
    return { ok: true, why: 'Bridge answers.' };
  } catch (e) {
    const err = e as BridgeError;
    return err.status === 401
      ? { ok: false, why: 'Bridge key is wrong. Open Settings.' }
      : { ok: false, why: err.message };
  }
}

/** Read the update manifest through the stored check. */
export async function checkUpdate(): Promise<HealthState> {
  return updateHealth(await runUpdateCheck());
}

/** Read the kept server key state. */
export function checkKey(): HealthState {
  return keyHealth(loadToken());
}
