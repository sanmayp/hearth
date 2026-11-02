/** Copy helper tests. Modern path, legacy path, and failure. */
import { describe, expect, it, vi, afterEach } from 'vitest';
import { copyText } from './copy';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('copyText', () => {
  it('uses the clipboard API when present', async () => {
    const writeText = vi.fn(async () => {});
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    await expect(copyText('lo-restart')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('lo-restart');
  });

  it('falls back to execCommand without clipboard', async () => {
    vi.stubGlobal('navigator', {});
    const area = { value: '', select: vi.fn(), remove: vi.fn() };
    vi.stubGlobal('document', {
      createElement: () => area,
      body: { appendChild: vi.fn() },
      execCommand: () => true
    });
    await expect(copyText('lo-check')).resolves.toBe(true);
    expect(area.value).toBe('lo-check');
  });

  it('returns false when nothing works', async () => {
    vi.stubGlobal('navigator', {});
    vi.stubGlobal('document', {
      createElement: () => {
        throw new Error('no dom');
      }
    });
    await expect(copyText('x')).resolves.toBe(false);
  });
});
