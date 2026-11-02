/** Health check tests. Reasons stay plain, keys stay hidden. */
import { describe, expect, it, vi, afterEach } from 'vitest';
import {
  checkBridge,
  checkKey,
  checkUpdate,
  dataHealth,
  keyHealth,
  updateHealth
} from './health';

afterEach(() => {
  vi.unstubAllGlobals();
});

function stubStorage(map = new Map<string, string>()): Map<string, string> {
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v)
  });
  return map;
}

describe('keyHealth', () => {
  it('is green with a kept key and red without', () => {
    expect(keyHealth('abc')).toEqual({ ok: true, why: 'Key kept.' });
    expect(keyHealth('')).toEqual({ ok: false, why: 'No server key. Open Settings.' });
  });

  it('never repeats the key value', () => {
    stubStorage(new Map([['hearth:token', 'secret-9']]));
    const h = checkKey();
    expect(h.ok).toBe(true);
    expect(h.why).not.toContain('secret-9');
  });
});

describe('dataHealth', () => {
  it('is green when live and keeps the error when red', () => {
    expect(dataHealth(true, '')).toEqual({ ok: true, why: 'Live.' });
    expect(dataHealth(false, 'No link to Hearth. Check your link and retry.')).toEqual({
      ok: false,
      why: 'No link to Hearth. Check your link and retry.'
    });
    expect(dataHealth(false, '')).toEqual({ ok: false, why: 'No link.' });
  });
});

describe('updateHealth', () => {
  it('is green for current and available states', () => {
    expect(updateHealth({ status: 'current', version: '0.2', detail: '' })).toEqual({
      ok: true,
      why: 'App is current.'
    });
    expect(updateHealth({ status: 'available', version: '0.2', detail: '' })).toEqual({
      ok: true,
      why: 'Update 0.2 ready.'
    });
  });

  it('is red with the stored reason otherwise', () => {
    expect(updateHealth({ status: 'error', version: '', detail: 'Manifest is bad.' })).toEqual(
      {
        ok: false,
        why: 'Manifest is bad.'
      }
    );
    expect(updateHealth({ status: 'unknown', version: '', detail: '' })).toEqual({
      ok: false,
      why: 'Update check failed.'
    });
  });
});

describe('checkBridge', () => {
  it('stays red without a key or link and never calls', async () => {
    stubStorage();
    const fetch = vi.fn();
    vi.stubGlobal('fetch', fetch);
    const h = await checkBridge();
    expect(h).toEqual({ ok: false, why: 'Add the bridge key in Settings.' });
    expect(fetch).not.toHaveBeenCalled();
  });

  it('is green when panes answer', async () => {
    stubStorage(new Map([['hearth:bridge-key', 'bkey']]));
    vi.stubGlobal('location', { search: '?bridge=http://127.0.0.1:9999' });
    vi.stubGlobal('fetch', async () => ({ ok: true, json: async () => ({ panes: [] }) }));
    const h = await checkBridge();
    expect(h).toEqual({ ok: true, why: 'Bridge answers.' });
  });

  it('names a wrong key on 401', async () => {
    stubStorage(new Map([['hearth:bridge-key', 'bad']]));
    vi.stubGlobal('location', { search: '?bridge=http://127.0.0.1:9999' });
    vi.stubGlobal('fetch', async () => ({
      ok: false,
      status: 401,
      json: async () => ({ message: 'Auth is missing or wrong.' })
    }));
    const h = await checkBridge();
    expect(h).toEqual({ ok: false, why: 'Bridge key is wrong. Open Settings.' });
  });
});

describe('checkUpdate', () => {
  it('is red outside the Android app with a plain reason', async () => {
    stubStorage();
    const h = await checkUpdate();
    expect(h.ok).toBe(false);
    expect(h.why).toContain('Android app');
  });
});
