/** Screen-use tests. Records, splits, throttle, and sync shapes. */
import { describe, expect, it, vi, afterEach } from 'vitest';
import {
  confirmScreen,
  dueForSync,
  isUsed,
  loadConfirms,
  loadUse,
  markSynced,
  recordOpen,
  splitScreens,
  syncOpen
} from './screenuse';

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

describe('recordOpen and loadUse', () => {
  it('keeps first earliest and moves last forward', () => {
    stubStorage();
    recordOpen('/threads', new Date(2026, 9, 15, 8, 0));
    const map = recordOpen('/threads', new Date(2026, 9, 15, 9, 30));
    expect(map['/threads']).toEqual({ first: '2026-10-15 08:00', last: '2026-10-15 09:30' });
    expect(loadUse()).toEqual(map);
  });

  it('reads junk storage as empty', () => {
    stubStorage(new Map([['hearth:screen-use', 'nope{']]));
    expect(loadUse()).toEqual({});
    expect(isUsed(loadUse(), '/threads')).toBe(false);
  });

  it('counts one open as used', () => {
    stubStorage();
    expect(isUsed(loadUse(), '/threads')).toBe(false);
    recordOpen('/threads');
    expect(isUsed(loadUse(), '/threads')).toBe(true);
  });
});

describe('splitScreens', () => {
  it('splits unopened and done in order', () => {
    stubStorage();
    recordOpen('/threads');
    expect(splitScreens(['/threads', '/tasks', '/'], loadUse())).toEqual({
      unopened: ['/tasks', '/'],
      done: ['/threads']
    });
  });
});

describe('dueForSync', () => {
  it('syncs missing and stale stamps only', () => {
    const now = new Date(2026, 9, 15, 10, 0);
    expect(dueForSync({}, '/threads', now)).toBe(true);
    expect(dueForSync({ '/threads': 'junk' }, '/threads', now)).toBe(true);
    expect(dueForSync({ '/threads': '2026-10-15 08:30' }, '/threads', now)).toBe(true);
    expect(dueForSync({ '/threads': '2026-10-15 09:30' }, '/threads', now)).toBe(false);
  });

  it('marks synced stamps through storage', () => {
    stubStorage();
    markSynced('/threads', new Date(2026, 9, 15, 9, 30));
    const map = JSON.parse(
      (globalThis.localStorage?.getItem('hearth:screen-sync') as string) ?? '{}'
    );
    expect(map['/threads']).toBe('2026-10-15 09:30');
  });
});

describe('syncOpen', () => {
  it('stays quiet without a key and never calls', async () => {
    stubStorage();
    const fetch = vi.fn();
    vi.stubGlobal('fetch', fetch);
    await expect(syncOpen('/threads', 'a', 'b')).resolves.toBe(false);
    expect(fetch).not.toHaveBeenCalled();
  });

  it('posts the record and reports the answer', async () => {
    stubStorage(new Map([['hearth:token', 'k']]));
    vi.stubGlobal('fetch', async () => ({ ok: true, json: async () => ({}) }));
    await expect(syncOpen('/threads', 'a', 'b')).resolves.toBe(true);
  });

  it('returns false when the link is down', async () => {
    stubStorage(new Map([['hearth:token', 'k']]));
    vi.stubGlobal('fetch', async () => {
      throw new Error('down');
    });
    await expect(syncOpen('/threads', 'a', 'b')).resolves.toBe(false);
  });
});

describe('confirmScreen', () => {
  it('returns the stamped verdict', async () => {
    stubStorage(new Map([['hearth:token', 'k']]));
    vi.stubGlobal('fetch', async () => ({
      ok: true,
      json: async () => ({ confirmed_at: '2026-11-02' })
    }));
    await expect(confirmScreen('/threads', 'works')).resolves.toEqual({
      ok: true,
      confirmed_at: '2026-11-02',
      error: ''
    });
  });

  it('carries a plain error when the post fails', async () => {
    stubStorage(new Map([['hearth:token', 'k']]));
    vi.stubGlobal('fetch', async () => {
      throw new Error('down');
    });
    const res = await confirmScreen('/threads', 'broken');
    expect(res.ok).toBe(false);
    expect(res.error).toContain('No link to Hearth');
  });
});

describe('loadConfirms', () => {
  it('keeps works and broken verdicts only', async () => {
    stubStorage(new Map([['hearth:token', 'k']]));
    vi.stubGlobal('fetch', async () => ({
      ok: true,
      json: async () => ({
        screens: {
          '/threads': { confirmed: 'works', confirmed_at: '2026-11-02' },
          '/tasks': { confirmed: 'maybe', confirmed_at: '' },
          '/bogus': null
        }
      })
    }));
    await expect(loadConfirms()).resolves.toEqual({
      '/threads': { confirmed: 'works', confirmed_at: '2026-11-02' }
    });
  });

  it('reads missing data as none', async () => {
    stubStorage();
    vi.stubGlobal('fetch', async () => {
      throw new Error('down');
    });
    await expect(loadConfirms()).resolves.toEqual({});
  });
});
