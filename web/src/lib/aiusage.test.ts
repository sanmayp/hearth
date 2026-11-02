import { describe, expect, it } from 'vitest';
import { barWidth, laneLabel, totalsLine, EMPTY_USAGE } from './aiusage';

describe('barWidth', () => {
  it('scales calls against the max', () => {
    expect(barWidth(5, 10)).toBe(50);
    expect(barWidth(10, 10)).toBe(100);
    expect(barWidth(20, 10)).toBe(100);
  });
  it('gives zero for empty input', () => {
    expect(barWidth(0, 10)).toBe(0);
    expect(barWidth(5, 0)).toBe(0);
  });
});

describe('totalsLine', () => {
  it('names zero runs plainly', () => {
    expect(totalsLine(EMPTY_USAGE.totals)).toBe('No runs yet.');
  });
  it('splits live and mock', () => {
    expect(
      totalsLine({ calls: 3, live: 2, mock: 1, tokens_in: 9, tokens_out: 3, errors: 0 })
    ).toBe('2 live calls, 1 mock call.');
  });
});

describe('laneLabel', () => {
  it('trims folders and lane prefixes', () => {
    expect(laneLabel('lanes/lane-atlas.txt')).toBe('atlas');
    expect(laneLabel('night-1.txt')).toBe('night-1');
  });
});
