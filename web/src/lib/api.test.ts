import { describe, expect, it, vi, afterEach } from 'vitest';
import { apiGet, apiPost, apiGetOr, ageLabel, saveStash, loadToken, saveToken } from './api';

afterEach(() => {
  vi.unstubAllGlobals();
});

/** Node test env has no storage. Stand in with a map. */
function stubStorage(fill = ''): void {
  const map = new Map<string, string>();
  if (fill) map.set('hearth:token', fill);
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v)
  });
}

describe('apiGet', () => {
  it('returns data plus time on ok', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: true, json: async () => ({ ok: true }) }))
    );
    const { data, at } = await apiGet<{ ok: boolean }>('/hello');
    expect(data.ok).toBe(true);
    expect(at).toBeInstanceOf(Date);
  });

  it('throws plain error on bad status', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 500 })));
    await expect(apiGet('/hello')).rejects.toMatchObject({ status: 500 });
  });

  it('throws plain error when offline', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new Error('down');
      })
    );
    await expect(apiGet('/hello')).rejects.toMatchObject({ status: 0 });
  });
});

describe('apiPost', () => {
  it('sends json and returns data', async () => {
    const fetchMock = vi.fn(async () => ({ ok: true, json: async () => ({ ok: true }) }));
    vi.stubGlobal('fetch', fetchMock);
    const out = await apiPost<{ ok: boolean }>('/reminders', { title: 'x' });
    expect(out.ok).toBe(true);
    expect(fetchMock).toHaveBeenCalledWith(
      '/v1/reminders',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('throws plain error when offline', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new Error('down');
      })
    );
    await expect(apiPost('/reminders', {})).rejects.toMatchObject({ status: 0 });
  });
});

describe('apiGetOr', () => {
  // Test env has no PUBLIC_HEARTH_API_BASE, so DEMO_ONLY is true.

  it('serves samples without fetch in demo', async () => {
    const fetchMock = vi.fn(async () => ({ ok: true, json: async () => ({ a: 1 }) }));
    vi.stubGlobal('fetch', fetchMock);
    const res = await apiGetOr('/demo-only', { a: 2 });
    expect(fetchMock).not.toHaveBeenCalled();
    expect(res.live).toBe(false);
    expect(res.stale).toBe(false);
    expect(res.data).toEqual({ a: 2 });
  });

  it('serves stash as stale without fetch in demo', async () => {
    const fetchMock = vi.fn(async () => {
      throw new Error('must not fetch');
    });
    vi.stubGlobal('fetch', fetchMock);
    saveStash('/stashed', { a: 9 });
    const res = await apiGetOr('/stashed', { a: 2 });
    expect(fetchMock).not.toHaveBeenCalled();
    expect(res.live).toBe(false);
    expect(res.stale).toBe(true);
    expect(res.data).toEqual({ a: 9 });
  });
});

describe('token', () => {
  it('keeps the key on this device only', () => {
    stubStorage();
    expect(loadToken()).toBe('');
    saveToken('k1');
    expect(loadToken()).toBe('k1');
  });

  it('sends the key on reads', async () => {
    stubStorage('k1');
    const fetchMock = vi.fn(async () => ({ ok: true, json: async () => ({ ok: true }) }));
    vi.stubGlobal('fetch', fetchMock);
    await apiGet('/hello');
    expect(fetchMock).toHaveBeenCalledWith(
      '/v1/hello',
      expect.objectContaining({ headers: { Authorization: 'Bearer k1' } })
    );
  });

  it('sends the key on writes', async () => {
    stubStorage('k1');
    const fetchMock = vi.fn(async () => ({ ok: true, json: async () => ({ ok: true }) }));
    vi.stubGlobal('fetch', fetchMock);
    await apiPost('/reminders', {});
    expect(fetchMock).toHaveBeenCalledWith(
      '/v1/reminders',
      expect.objectContaining({
        headers: { 'Content-Type': 'application/json', Authorization: 'Bearer k1' }
      })
    );
  });

  it('sends no key when none is kept', async () => {
    stubStorage();
    const fetchMock = vi.fn(async () => ({ ok: true, json: async () => ({ ok: true }) }));
    vi.stubGlobal('fetch', fetchMock);
    await apiGet('/hello');
    expect(fetchMock).toHaveBeenCalledWith('/v1/hello', expect.objectContaining({ headers: {} }));
  });

  it('points to Settings on 401', async () => {
    stubStorage();
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 401 })));
    await expect(apiGet('/hello')).rejects.toMatchObject({ status: 401, message: expect.stringContaining('Settings') });
    await expect(apiPost('/reminders', {})).rejects.toMatchObject({ status: 401, message: expect.stringContaining('Settings') });
  });
});

describe('ageLabel', () => {
  it('says just now under a minute', () => {
    const now = new Date('2026-10-31T12:00:30Z');
    expect(ageLabel(new Date('2026-10-31T12:00:00Z'), now)).toBe('just now');
  });

  it('says minutes under an hour', () => {
    const now = new Date('2026-10-31T12:05:00Z');
    expect(ageLabel(new Date('2026-10-31T12:00:00Z'), now)).toBe('5 min ago');
  });

  it('marks old data stale', () => {
    const now = new Date('2026-11-02T12:00:00Z');
    const label = ageLabel(new Date('2026-10-31T12:00:00Z'), now);
    expect(label.startsWith('stale since')).toBe(true);
  });
});
