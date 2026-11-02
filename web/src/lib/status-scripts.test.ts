import { describe, expect, it } from 'vitest';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

// Prebuild status scripts must pass in the public copy, where the
// private factory/ dir does not exist. They fall back to empty data.
describe('status scripts without factory dir', () => {
  const webRoot = join(__dirname, '..', '..');
  const run = (script: string) =>
    execFileSync('node', [join(webRoot, 'scripts', script)], { encoding: 'utf8' });

  it('factory-status exits zero and writes valid JSON', () => {
    run('factory-status.mjs');
    const out = join(webRoot, 'static', 'factory-status.json');
    expect(existsSync(out)).toBe(true);
    const data = JSON.parse(readFileSync(out, 'utf8')) as {
      reqs: unknown[];
      state: object;
      ledger: unknown[];
    };
    expect(Array.isArray(data.reqs)).toBe(true);
    expect(data.state).toEqual({});
    expect(data.ledger).toEqual([]);
  });

  it('builds-status exits zero and writes valid JSON', () => {
    run('builds-status.mjs');
    const out = join(webRoot, 'static', 'builds-status.json');
    expect(existsSync(out)).toBe(true);
    const data = JSON.parse(readFileSync(out, 'utf8')) as { builds: unknown[] };
    expect(data.builds).toEqual([]);
  });
});
