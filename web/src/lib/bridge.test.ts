/** Bridge client tests. Confirm flow, errors, and key rules. */
import { describe, expect, it, vi, afterEach } from 'vitest';
import {
  answerError,
  approveState,
  BRIDGE_ESC,
  bridgeError,
  bridgeGet,
  bridgePost,
  buildBridgeBase,
  codeExpired,
  codeMatches,
  loadBridgeKey,
  loadBridgePort,
  previewLabel,
  QUICK_ACTIONS,
  saveBridgeKey,
  saveBridgePort
} from './bridge';

afterEach(() => {
  vi.unstubAllGlobals();
});

/** Node test env has no storage. Stand in with a map. */
function stubStorage(map = new Map<string, string>()): Map<string, string> {
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v)
  });
  return map;
}

describe('buildBridgeBase', () => {
  it('returns null in demo mode', () => {
    expect(buildBridgeBase('demo', '8449', 'https:', 'x.ts.net')).toBeNull();
  });

  it('uses the page host for same-origin mode', () => {
    expect(buildBridgeBase('same', '8449', 'https:', 'host.tail')).toBe(
      'https://host.tail:8449'
    );
  });

  it('swaps the API host port for the bridge port', () => {
    expect(buildBridgeBase('https://host.tail:8187', '8449', '', '')).toBe(
      'https://host.tail:8449'
    );
  });

  it('returns null for junk bases', () => {
    expect(buildBridgeBase('notaurl', '8449', '', '')).toBeNull();
  });
});

describe('bridge key and port storage', () => {
  it('keeps the key separate from the server key', () => {
    const map = stubStorage();
    saveBridgeKey('bkey');
    expect(map.get('hearth:bridge-key')).toBe('bkey');
    expect(map.has('hearth:token')).toBe(false);
    expect(loadBridgeKey()).toBe('bkey');
  });

  it('falls back to 8449 on bad ports', () => {
    stubStorage(new Map([['hearth:bridge-port', 'abc']]));
    expect(loadBridgePort()).toBe('8449');
    stubStorage(new Map([['hearth:bridge-port', '99999']]));
    expect(loadBridgePort()).toBe('8449');
    stubStorage(new Map([['hearth:bridge-port', '8200']]));
    expect(loadBridgePort()).toBe('8200');
  });

  it('saves the port as typed', () => {
    const map = stubStorage();
    saveBridgePort('8201');
    expect(map.get('hearth:bridge-port')).toBe('8201');
  });
});

describe('answerError', () => {
  it('rejects empty, split paragraphs, and long text', () => {
    expect(answerError('   ')).toBe('Write a line first.');
    expect(answerError('a\n\nb')).toBe('One paragraph only. No blank lines.');
    expect(answerError('x'.repeat(1001))).toBe('Keep 1000 chars or less.');
  });

  it('accepts one paragraph up to 1000 chars', () => {
    expect(answerError('ship it')).toBeNull();
    expect(answerError('line one\nline two')).toBeNull();
    expect(answerError('x'.repeat(1000))).toBeNull();
  });
});

describe('quick actions', () => {
  it('lists digits, answers, escape, and continue', () => {
    expect(QUICK_ACTIONS.map((q) => q.label)).toEqual([
      '1',
      '2',
      '3',
      '4',
      'Yes',
      'No',
      'Esc',
      'Continue'
    ]);
    expect(QUICK_ACTIONS.map((q) => q.text)).toEqual([
      '1',
      '2',
      '3',
      '4',
      'y',
      'n',
      BRIDGE_ESC,
      'continue'
    ]);
  });

  it('labels control bytes and empty text plainly', () => {
    expect(previewLabel(BRIDGE_ESC)).toBe('(Escape key)');
    expect(previewLabel('')).toBe('(Enter press, no text)');
    expect(previewLabel('hi')).toBe('hi');
  });
});

describe('codeMatches', () => {
  it('matches exact codes and trims space', () => {
    expect(codeMatches('AB12', 'AB12')).toBe(true);
    expect(codeMatches('  AB12 ', 'AB12')).toBe(true);
    expect(codeMatches('AB12', 'CD34')).toBe(false);
    expect(codeMatches('', '')).toBe(false);
  });
});

describe('codeExpired', () => {
  it('expires after the window passes', () => {
    expect(codeExpired(1000, 60, 1000 + 59_999)).toBe(false);
    expect(codeExpired(1000, 60, 1000 + 60_000)).toBe(true);
  });
});

describe('bridgeError', () => {
  it('keeps the exact server text', async () => {
    const res = { status: 403, json: async () => ({ message: 'Pane is not allowed.' }) };
    await expect(bridgeError(res as Response)).resolves.toMatchObject({
      message: 'Pane is not allowed.',
      status: 403,
      retryAfter: null
    });
  });

  it('carries the retry time on 429', async () => {
    const res = {
      status: 429,
      json: async () => ({ message: 'Rate limit hit. Retry in 12 s.', retry_after: 12 })
    };
    await expect(bridgeError(res as Response)).resolves.toMatchObject({
      retryAfter: 12,
      status: 429
    });
  });

  it('falls back when the body is unreadable', async () => {
    const res = {
      status: 500,
      json: async () => {
        throw new Error('bad');
      }
    };
    await expect(bridgeError(res as unknown as Response)).resolves.toMatchObject({
      status: 500
    });
  });
});

describe('bridgeGet and bridgePost', () => {
  it('refuses without a key or link', async () => {
    stubStorage();
    await expect(bridgeGet('/v1/panes')).rejects.toMatchObject({ status: 0 });
    await expect(bridgePost('/v1/panes/x/message', {})).rejects.toMatchObject({ status: 0 });
  });
});

describe('approveState', () => {
  it('enables Approve only on a waiting approval', () => {
    expect(approveState(true, true)).toEqual({ disabled: false, reason: '' });
  });

  it('holds Approve with a visible reason otherwise', () => {
    expect(approveState(false, true).disabled).toBe(true);
    expect(approveState(true, false)).toEqual({ disabled: true, reason: 'Nothing to approve.' });
    expect(approveState(true, null).reason).toContain('unknown');
  });
});
