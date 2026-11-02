import { describe, expect, it } from 'vitest';
import {
  barShare,
  buildRows,
  dateLine,
  dayCounts,
  doneToday,
  greeting,
  isStalled,
  ringFrac,
  ringOffset,
  stepCounts,
  type HomeRow
} from './home';
import type { FeatureRow } from './features';
import type { TodayView } from './today';

function view(over: Partial<TodayView> = {}): TodayView {
  return {
    agenda: [],
    reminders_due: [],
    tasks_due: [],
    business_alerts: [],
    health_checkin_done: false,
    next_action: null,
    updated_at: null,
    ...over
  };
}

describe('greeting', () => {
  it('greets by hour', () => {
    expect(greeting(8)).toBe('Good morning');
    expect(greeting(13)).toBe('Good afternoon');
    expect(greeting(20)).toBe('Good evening');
  });

  it('prints the date line', () => {
    expect(dateLine(new Date(2026, 10, 2))).toBe('Monday, November 2');
  });
});

describe('dayCounts', () => {
  it('adds open and done across lists', () => {
    const c = dayCounts(
      view({
        reminders_due: [{ id: 'r1', title: 'Pills', at: null }],
        tasks_due: [{ id: 't1', title: 'Call', at: null }],
        health_checkin_done: true
      }),
      2,
      1,
      0
    );
    expect(c.done).toBe(2);
    expect(c.total).toBe(6);
    expect(c.waiting).toBe(4);
  });

  it('counts past-due reminders as stalled', () => {
    const c = dayCounts(
      view({ reminders_due: [{ id: 'r1', title: 'Late', at: '2020-01-01T00:00:00Z' }] }),
      0,
      0,
      0,
      new Date('2026-11-02T00:00:00Z').getTime()
    );
    expect(c.stalled).toBe(1);
  });

  it('never reads NaN on an empty day', () => {
    const c = dayCounts(view({ health_checkin_done: true }), 0, 0, 0);
    expect(c.total).toBe(1);
    expect(ringFrac(c.done, c.total)).toBe(1);
    expect(ringFrac(0, 0)).toBe(0);
  });
});

describe('ring and bars', () => {
  it('sweeps the mockup ring geometry', () => {
    expect(ringOffset(0)).toBeCloseTo(552.9, 1);
    expect(ringOffset(1)).toBeCloseTo(0, 1);
    expect(ringOffset(3 / 7)).toBeCloseTo(316, 0);
  });

  it('shares bars off the biggest count', () => {
    expect(barShare(1, 4)).toBe(25);
    expect(barShare(4, 4)).toBe(100);
    expect(barShare(0, 0)).toBe(0);
  });

  it('counts log dates as today only', () => {
    expect(doneToday({ 'task:1': '2026-11-02', 'task:2': '2026-11-01' }, '2026-11-02')).toBe(1);
  });

  it('stalls only past stamps', () => {
    const now = new Date('2026-11-02T12:00:00Z').getTime();
    expect(isStalled('2026-11-02T11:00:00Z', now)).toBe(true);
    expect(isStalled('2026-11-02T13:00:00Z', now)).toBe(false);
    expect(isStalled(null, now)).toBe(false);
    expect(isStalled('junk', now)).toBe(false);
  });
});

describe('buildRows', () => {
  const feat: FeatureRow = {
    id: 'f1',
    name: 'Fix install',
    area: 'Installer',
    stage: 'built',
    where: 'Needs You',
    tested: '',
    deployed: '',
    last_used: '',
    next: '',
    steps: [
      { text: 'Old step.', owner: 'you', done: true, done_at: '2026-11-02' },
      { text: 'Next step.', owner: 'you', done: false }
    ]
  };

  it('mixes lists with tags and hides done on ask', () => {
    const v = view({
      reminders_due: [{ id: 'r1', title: 'Pills', at: null }],
      tasks_due: [{ id: 't1', title: 'Call', at: null }]
    });
    const moves = [
      {
        featureId: 'f1',
        featureName: 'Fix install',
        area: 'Installer',
        stepIndex: 1,
        text: 'Next step.',
        stepSummary: null,
        where: '',
        how: '',
        cmd: ''
      }
    ];
    const rows = buildRows(v, [feat], moves, '2026-11-02', false);
    expect(rows.map((r: HomeRow) => r.kind)).toEqual([
      'reminder',
      'task',
      'health',
      'step',
      'step'
    ]);
    expect(rows[3].tag).toBe('dev');
    expect(rows[4].done).toBe(true);
    const hidden = buildRows(v, [feat], moves, '2026-11-02', true);
    expect(hidden.some((r) => r.done)).toBe(false);
  });

  it('skips the health row after check-in', () => {
    const rows = buildRows(view({ health_checkin_done: true }), [], [], '2026-11-02', false);
    expect(rows).toEqual([]);
  });

  it('counts open and today steps', () => {
    const moves = [
      {
        featureId: 'f1',
        featureName: 'Fix install',
        area: '',
        stepIndex: 1,
        text: 'Next step.',
        stepSummary: null,
        where: '',
        how: '',
        cmd: ''
      }
    ];
    expect(stepCounts(moves, [feat], '2026-11-02')).toEqual({ open: 1, doneToday: 1 });
    expect(stepCounts([], [feat], '2026-11-01')).toEqual({ open: 0, doneToday: 0 });
  });
});
