import { describe, expect, it } from 'vitest';
import { checkFailureDetail } from './update';

describe('checkFailureDetail', () => {
  it('ships the core cause as-is', () => {
    expect(checkFailureDetail('signature mismatch')).toBe('signature mismatch');
    expect(checkFailureDetail('updates are not set up in this build')).toBe(
      'updates are not set up in this build'
    );
  });

  it('reads error objects and falls back plain', () => {
    expect(checkFailureDetail(new Error('no link'))).toBe('no link');
    expect(checkFailureDetail(null)).toBe('Update check failed. Check the link and retry.');
    expect(checkFailureDetail('   ')).toBe('Update check failed. Check the link and retry.');
  });

  it('caps long causes', () => {
    expect(checkFailureDetail('x'.repeat(300))).toHaveLength(200);
  });
});
