import { describe, expect, it } from 'vitest';
import {
  answerLine,
  approvalUnknown,
  bridgeOff,
  checkPlain,
  commitLine,
  laneLine,
  noLink,
  nothingToApprove,
  progressUnknown,
  resultLine,
  sinceLine,
  spendLine,
  staleLine,
  waitLine,
  wordCount
} from './plain';

describe('checkPlain', () => {
  it('rejects long sentences', () => {
    const long = Array(26).fill('word').join(' ');
    expect(checkPlain(long)).toHaveLength(1);
    expect(checkPlain('Short line.', 20)).toEqual([]);
  });

  it('rejects banned words', () => {
    expect(checkPlain('Please utilize the bridge.')).toContain('banned word: utilize');
    expect(checkPlain('Check the bridge.')).toEqual([]);
  });
});

describe('templates ship plain', () => {
  const samples = [
    laneLine('Prices', 'loads daily price files', 'working', 'hands data to Earnings'),
    laneLine('Features', 'builds the Features screen', 'waiting for you', ''),
    waitLine('Bars'),
    answerLine(1),
    answerLine(3),
    nothingToApprove(),
    approvalUnknown(),
    bridgeOff(),
    noLink(),
    staleLine(22),
    resultLine(212, 0),
    spendLine('tool calls 22 of 150'),
    progressUnknown(),
    commitLine('a1b2c3d', '12 min'),
    sinceLine('It builds the screen', '12 min')
  ];

  it('every template passes the checker', () => {
    for (const s of samples) expect(checkPlain(s)).toEqual([]);
  });

  it('the lane pattern reads as specified', () => {
    expect(samples[0]).toBe(
      'Prices loads daily price files. It is working. It hands data to Earnings.'
    );
    expect(wordCount(samples[0].split('.')[0])).toBeLessThanOrEqual(25);
  });
});
