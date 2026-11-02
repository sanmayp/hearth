/** Refresh lib tests. Gesture rules plus the stamp shape. */
import { describe, expect, it, vi, afterEach } from 'vitest';
import { pullToRefresh, updatedStamp, PULL_THRESHOLD } from './refresh';

afterEach(() => {
  vi.unstubAllGlobals();
});

type FakeNode = {
  handlers: Map<string, (e: unknown) => void>;
  addEventListener: (t: string, h: (e: unknown) => void) => void;
  removeEventListener: (t: string) => void;
};

function fakeNode(): FakeNode {
  const handlers = new Map<string, (e: unknown) => void>();
  return {
    handlers,
    addEventListener: (t, h) => void handlers.set(t, h),
    removeEventListener: (t) => void handlers.delete(t)
  };
}

function touch(clientY: number): unknown {
  return { touches: [{ clientY }], changedTouches: [{ clientY }] };
}

describe('pullToRefresh', () => {
  it('fires after a long pull at the top', () => {
    vi.stubGlobal('window', { scrollY: 0 });
    const node = fakeNode();
    let calls = 0;
    pullToRefresh(node as unknown as HTMLElement, () => (calls += 1));
    node.handlers.get('touchstart')?.(touch(100));
    node.handlers.get('touchend')?.(touch(100 + PULL_THRESHOLD));
    expect(calls).toBe(1);
  });

  it('ignores short pulls, scrolled pages, and multi-touch', () => {
    vi.stubGlobal('window', { scrollY: 0 });
    const node = fakeNode();
    let calls = 0;
    pullToRefresh(node as unknown as HTMLElement, () => (calls += 1));
    const start = node.handlers.get('touchstart');
    const end = node.handlers.get('touchend');
    start?.(touch(100));
    end?.(touch(100 + PULL_THRESHOLD - 1));
    start?.({ touches: [{ clientY: 100 }, { clientY: 120 }], changedTouches: [] });
    end?.(touch(100 + PULL_THRESHOLD + 50));
    expect(calls).toBe(0);
    vi.stubGlobal('window', { scrollY: 40 });
    start?.(touch(100));
    end?.(touch(100 + PULL_THRESHOLD + 50));
    expect(calls).toBe(0);
  });

  it('removes listeners on destroy', () => {
    vi.stubGlobal('window', { scrollY: 0 });
    const node = fakeNode();
    const action = pullToRefresh(node as unknown as HTMLElement, () => {});
    expect(node.handlers.size).toBe(2);
    action.destroy();
    expect(node.handlers.size).toBe(0);
  });
});

describe('updatedStamp', () => {
  it('writes zero-padded local time', () => {
    expect(updatedStamp(new Date(2026, 9, 15, 14, 32))).toBe('Updated 14:32.');
    expect(updatedStamp(new Date(2026, 9, 15, 9, 5))).toBe('Updated 09:05.');
  });

  it('names a missing stamp plainly', () => {
    expect(updatedStamp(null)).toBe('Never updated.');
  });
});
