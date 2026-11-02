import { describe, expect, it } from 'vitest';
import {
  activityLine,
  arcLabel,
  chipCounts,
  hasFlow,
  hexTop,
  laneColor,
  lastCommitFor,
  LANE_TONE,
  matchLane,
  minsSince,
  nodeState,
  ribbonPath,
  ribbonWidth,
  selectSlots,
  stampHM,
  timelineRows,
  staleWorking,
  type LaneMeta
} from './dev';
import type { NeedThread } from './needs-you';

function thread(over: Partial<NeedThread> = {}): NeedThread {
  return {
    name: 'prices',
    pane: 'p1',
    state: 'working',
    at: null,
    changed_at: null,
    alerts: [],
    question: null,
    ...over
  };
}

function lane(over: Partial<LaneMeta> = {}): LaneMeta {
  return { lane: 'prices', at: '', note: '', rows: null, errors: null, pct: null, spend: '', ...over };
}

describe('nodeState', () => {
  it('maps questions, work, blocks, idle, done', () => {
    expect(nodeState(thread({ question: 'Which source?' })).key).toBe('waiting');
    expect(nodeState(thread({ question: 'Which source?' })).reason).toBe('Which source?');
    expect(nodeState(thread()).key).toBe('working');
    expect(nodeState(thread({ state: 'blocked', alerts: ['Key is bad.'] })).reason).toBe(
      'Key is bad.'
    );
    expect(nodeState(thread({ state: 'idle' })).key).toBe('idle');
    expect(nodeState(thread({ state: 'done' })).key).toBe('done');
    expect(nodeState(thread({ state: 'gone' })).key).toBe('blocked');
  });
});

describe('matchLane and activity', () => {
  it('matches lane names loosely', () => {
    expect(matchLane(thread(), [lane()])?.lane).toBe('prices');
    expect(matchLane(thread({ name: 'other', pane: 'px' }), [lane()])).toBeNull();
  });

  it('prefers alerts over screen over note', () => {
    const t = thread({ alerts: ['Key is bad.'] });
    expect(activityLine(t, lane({ note: 'note' }), 'screen')).toBe('Key is bad.');
    expect(activityLine(thread(), lane({ note: 'note' }), 'screen')).toBe('screen');
    expect(activityLine(thread(), lane({ note: 'note' }), '')).toBe('note');
    expect(activityLine(thread(), null, '')).toBe('Unknown.');
  });
});

describe('stale and time', () => {
  it('flags quiet workers after 15 min', () => {
    const now = new Date('2026-11-02T12:00:00');
    expect(staleWorking(thread({ changed_at: '2026-11-02 11:44' }), now)).toBe(true);
    expect(staleWorking(thread({ changed_at: '2026-11-02 11:50' }), now)).toBe(false);
    expect(staleWorking(thread({ state: 'idle', changed_at: '2026-11-01 10:00' }), now)).toBe(
      false
    );
    expect(staleWorking(thread(), now)).toBe(false);
  });

  it('counts minutes and stamps hours', () => {
    expect(minsSince('2026-11-02 11:30', new Date('2026-11-02T12:00:00'))).toBe(30);
    expect(minsSince(null)).toBeNull();
    expect(stampHM(new Date('2026-11-02T08:05:00'))).toBe('08:05');
  });
});

describe('arcs and commits', () => {
  it('labels waits on you and on lanes', () => {
    expect(arcLabel(thread({ question: 'Q?' }), [])).toBe('waits on you');
    expect(arcLabel(thread({ alerts: ['waits on bars'] }), [lane({ lane: 'bars' })])).toBe(
      'waits on bars'
    );
    expect(arcLabel(thread(), [])).toBe('');
  });

  it('finds lane commits by message', () => {
    const commits = [
      { sha: 'aaa', msg: 'Fix prices load', at: '2026-11-02T10:00:00' },
      { sha: 'bbb', msg: 'Other work', at: '2026-11-02T11:00:00' }
    ];
    expect(lastCommitFor('prices', commits)?.sha).toBe('aaa');
    expect(lastCommitFor('macro', commits)).toBeNull();
  });

  it('scales ribbons and paths', () => {
    expect(ribbonWidth(0)).toBe(6);
    expect(ribbonWidth(9)).toBe(14);
    expect(ribbonPath(0, 3)).toMatch(/^M84 72 C/);
    expect(ribbonPath(1, 3)).toMatch(/^M276 72 C/);
    expect(ribbonPath(0, 6, 568)).toContain('568');
    expect(hexTop(4)).toBe(392);
    expect(hexTop(6)).toBe(568);
  });

  it('maps each state to one color', () => {
    const base = { idleMins: null, stale: false, approval: false };
    expect(laneColor({ ...base, state: 'working' })).toBe('green');
    expect(laneColor({ ...base, state: 'done' })).toBe('green');
    expect(laneColor({ ...base, state: 'waiting' })).toBe('yellow');
    expect(laneColor({ ...base, state: 'working', approval: true })).toBe('yellow');
    expect(laneColor({ ...base, state: 'idle', idleMins: 31 })).toBe('yellow');
    expect(laneColor({ ...base, state: 'idle', idleMins: 5 })).toBe('grey');
    expect(laneColor({ ...base, state: 'idle' })).toBe('grey');
    expect(laneColor({ ...base, state: 'blocked' })).toBe('red');
    expect(laneColor({ ...base, state: 'working', stale: true })).toBe('red');
    expect(LANE_TONE.green).toBe('var(--ok)');
  });

  it('runs dots only on healthy working lanes', () => {
    const base = { idleMins: null, stale: false, approval: false };
    expect(hasFlow({ ...base, state: 'working' })).toBe(true);
    expect(hasFlow({ ...base, state: 'idle' })).toBe(false);
    expect(hasFlow({ ...base, state: 'done' })).toBe(false);
    expect(hasFlow({ ...base, state: 'waiting' })).toBe(false);
    expect(hasFlow({ ...base, state: 'blocked' })).toBe(false);
    expect(hasFlow({ ...base, state: 'working', stale: true })).toBe(false);
    expect(hasFlow({ ...base, state: 'working', approval: true })).toBe(false);
  });

  it('slots 19 lanes with waiting never hidden', () => {
    const items = [
      ...Array.from({ length: 3 }, (_, i) => ({ id: `q${i}`, state: 'waiting' as const })),
      ...Array.from({ length: 2 }, (_, i) => ({ id: `b${i}`, state: 'blocked' as const })),
      ...Array.from({ length: 8 }, (_, i) => ({ id: `w${i}`, state: 'working' as const })),
      ...Array.from({ length: 4 }, (_, i) => ({ id: `i${i}`, state: 'idle' as const })),
      ...Array.from({ length: 2 }, (_, i) => ({ id: `d${i}`, state: 'done' as const }))
    ];
    const pick = selectSlots(items);
    expect(pick.shown).toHaveLength(12);
    expect(pick.done).toBe(2);
    expect(pick.more).toBe(5);
    expect(pick.shown.slice(0, 3)).toEqual(['q0', 'q1', 'q2']);
    expect(pick.shown.filter((s) => s.startsWith('w'))).toHaveLength(6);
    expect(pick.shown.some((s) => s.startsWith('d'))).toBe(false);
  });
});

describe('chips and timeline', () => {
  it('counts chips from threads plus approvals', () => {
    const c = chipCounts(
      [
        thread(),
        thread({ state: 'blocked' }),
        thread({ state: 'idle', question: 'Q?' }),
        thread({ state: 'done' })
      ],
      2
    );
    expect(c).toEqual({ stalled: 1, need: 3, building: 1 });
  });

  it('merges tests into the timeline newest first', () => {
    const rows = timelineRows(
      [{ sha: 'aaa', msg: 'Old', at: '2026-11-01T10:00:00' }],
      { tests: 30, failures: 1, errors: 0, at: '2026-11-02T10:00:00' }
    );
    expect(rows[0].kind).toBe('tests');
    expect(rows[0].title).toContain('FAIL');
    const pass = timelineRows([], { tests: 30, failures: 0, errors: 0, at: '' });
    expect(pass[0].title).toContain('PASS');
  });
});
