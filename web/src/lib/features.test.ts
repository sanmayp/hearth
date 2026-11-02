/** Features ledger tests: owner confirm, proof, quiet 24h flag, attention rules, area grouping, done reverse sort, URL parsing. */
import { describe, expect, it } from 'vitest';
import {
  attentionList,
  claimedLabel,
  doneList,
  groupByArea,
  inProgressList,
  isDone,
  isQuiet24h,
  needsAttention,
  ownerConfirmLabel,
  ownerConfirmTone,
  proofText,
  screenRouteFor,
  sortDoneNewestFirst,
  todayIso,
  splitWhereLinks,
  stageTone,
  stepProgressLabel,
  yourMoveList,
  type FeatureRow
} from './features';

function row(over: Partial<FeatureRow>): FeatureRow {
  return {
    id: 'f1',
    name: 'Sample feature',
    area: 'LifeOS app',
    stage: 'idea',
    where: 'Hearth app',
    tested: 'not tested',
    deployed: 'not deployed',
    last_used: 'never',
    next: 'none',
    completed_at: '',
    ...over
  };
}

describe('claimedLabel and ownerConfirmLabel', () => {
  it('formats claimed stage and side-by-side owner confirmation', () => {
    const f = row({
      stage: 'deployed',
      owner_confirmed: 'works',
      owner_confirmed_at: '2026-11-02'
    });
    expect(claimedLabel(f)).toBe('claimed: deployed');
    expect(ownerConfirmLabel(f)).toBe('you: works 11-02');
    expect(ownerConfirmTone(f)).toBe('var(--ok)');
  });

  it('formats broken and unconfirmed rows', () => {
    const broken = row({
      stage: 'in_use',
      owner_confirmed: 'broken',
      owner_confirmed_at: '2026-11-02 06:15'
    });
    expect(ownerConfirmLabel(broken)).toBe('you: broken 11-02');
    expect(ownerConfirmTone(broken)).toBe('var(--bad)');

    const unconfirmed = row({ stage: 'built' });
    expect(ownerConfirmLabel(unconfirmed)).toBe('you: unconfirmed');
    expect(ownerConfirmTone(unconfirmed)).toBe('var(--paper-dim)');
  });
});

describe('proofText', () => {
  it('returns trimmed proof when present and null when empty or missing', () => {
    expect(proofText(row({ proof: '58 unit tests by Muse (2026-11-02)' }))).toBe(
      '58 unit tests by Muse (2026-11-02)'
    );
    expect(proofText(row({ proof: '   ' }))).toBeNull();
    expect(proofText(row({ proof: undefined }))).toBeNull();
  });
});

describe('isQuiet24h', () => {
  const now = new Date(2026, 10, 2, 12, 0); // 2026-11-02 12:00 local

  it('flags last_used older than 24 hours, never, or unknown', () => {
    expect(isQuiet24h(row({ last_used: 'never' }), now)).toBe(true);
    expect(isQuiet24h(row({ last_used: 'unknown' }), now)).toBe(true);
    expect(isQuiet24h(row({ last_used: '2026-10-31' }), now)).toBe(true);
    expect(isQuiet24h(row({ last_used: '2026-11-01 08:00' }), now)).toBe(true);
  });

  it('clears flag when last_used is within 24 hours', () => {
    expect(isQuiet24h(row({ last_used: '2026-11-02' }), now)).toBe(false);
    expect(isQuiet24h(row({ last_used: '2026-11-01 18:00' }), now)).toBe(false);
  });
});

describe('needsAttention', () => {
  it('flags built, tested, unconfirmed deployed/in_use, and owner-broken rows', () => {
    expect(needsAttention(row({ stage: 'built' }))).toBe(true);
    expect(needsAttention(row({ stage: 'tested' }))).toBe(true);
    expect(needsAttention(row({ stage: 'deployed', last_used: '2026-11-02' }))).toBe(true);
    expect(needsAttention(row({ stage: 'in_use', owner_confirmed: 'broken' }))).toBe(true);
  });

  it('clears attention when owner confirmed works, and skips idea/building unless broken', () => {
    expect(
      needsAttention(row({ stage: 'deployed', owner_confirmed: 'works', last_used: '2026-11-02' }))
    ).toBe(false);
    expect(needsAttention(row({ stage: 'building' }))).toBe(false);
    expect(needsAttention(row({ stage: 'idea' }))).toBe(false);
  });

  it('attentionList keeps matching rows in order', () => {
    const rows = [
      row({ id: 'a', stage: 'in_use', owner_confirmed: 'works' }),
      row({ id: 'b', stage: 'deployed', last_used: 'never' }),
      row({ id: 'c', stage: 'tested' }),
      row({ id: 'd', stage: 'built' }),
      row({ id: 'e', stage: 'building' })
    ];
    expect(attentionList(rows).map((r) => r.id)).toEqual(['b', 'c', 'd']);
  });
});

describe('isDone, inProgressList, and doneList reverse sort', () => {
  it('counts a feature as done ONLY when owner confirms works', () => {
    expect(isDone(row({ stage: 'deployed' }))).toBe(false);
    expect(isDone(row({ stage: 'in_use', completed_at: '2026-11-01 23:18' }))).toBe(false);
    expect(
      isDone(row({ stage: 'in_use', owner_confirmed: 'broken', completed_at: '2026-11-01 23:18' }))
    ).toBe(false);
    expect(isDone(row({ stage: 'deployed', owner_confirmed: 'works' }))).toBe(true);
    expect(isDone(row({ stage: 'in_use', owner_confirmed: 'works' }))).toBe(true);
  });

  it('sorts done items newest completed_at first, with missing completed_at last', () => {
    const items = [
      row({ id: 'no-stamp', stage: 'deployed', owner_confirmed: 'works', completed_at: '' }),
      row({
        id: 'older',
        stage: 'in_use',
        owner_confirmed: 'works',
        completed_at: '2026-11-01 13:00'
      }),
      row({
        id: 'newest',
        stage: 'in_use',
        owner_confirmed: 'works',
        completed_at: '2026-11-01 23:18'
      }),
      row({
        id: 'middle',
        stage: 'deployed',
        owner_confirmed: 'works',
        completed_at: '2026-11-01 21:50'
      }),
      row({
        id: 'no-stamp-2',
        stage: 'deployed',
        owner_confirmed: 'works',
        completed_at: undefined
      })
    ];
    expect(sortDoneNewestFirst(items).map((r) => r.id)).toEqual([
      'newest',
      'middle',
      'older',
      'no-stamp',
      'no-stamp-2'
    ]);
    expect(doneList(items).map((r) => r.id)).toEqual([
      'newest',
      'middle',
      'older',
      'no-stamp',
      'no-stamp-2'
    ]);
  });

  it('inProgressList keeps non-done items (including unconfirmed deployed)', () => {
    const items = [
      row({
        id: 'done-1',
        stage: 'in_use',
        owner_confirmed: 'works',
        completed_at: '2026-11-01 23:18'
      }),
      row({ id: 'unconf-1', stage: 'deployed', completed_at: '2026-11-01 22:00' }),
      row({ id: 'prog-1', stage: 'building' }),
      row({ id: 'prog-2', stage: 'tested' })
    ];
    expect(inProgressList(items).map((r) => r.id)).toEqual(['unconf-1', 'prog-1', 'prog-2']);
  });
});

describe('groupByArea and stageTone', () => {
  it('groups features by area in encounter order', () => {
    const items = [
      row({ id: '1', area: 'LifeOS app' }),
      row({ id: '2', area: 'Tank' }),
      row({ id: '3', area: 'LifeOS app' })
    ];
    const groups = groupByArea(items);
    expect(groups.map((g) => g.area)).toEqual(['LifeOS app', 'Tank']);
    expect(groups[0].items.map((i) => i.id)).toEqual(['1', '3']);
    expect(groups[1].items.map((i) => i.id)).toEqual(['2']);
  });

  it('assigns distinct stage tones for all six stages', () => {
    expect(stageTone('idea')).toBe('var(--paper-dim)');
    expect(stageTone('building')).toBe('var(--hue-reminders)');
    expect(stageTone('built')).toBe('var(--warn)');
    expect(stageTone('tested')).toBe('var(--hue-business)');
    expect(stageTone('deployed')).toBe('var(--ok)');
    expect(stageTone('in_use')).toBe('#6de36d');
  });
});

describe('splitWhereLinks', () => {
  it('extracts https URLs without trailing punctuation', () => {
    const parts = splitWhereLinks(
      'APK: https://example.ts.net:8448/hearth.apk. Code in repo.'
    );
    expect(parts).toEqual([
      { text: 'APK: ', href: null },
      {
        text: 'https://example.ts.net:8448/hearth.apk',
        href: 'https://example.ts.net:8448/hearth.apk'
      },
      { text: '. Code in repo.', href: null }
    ]);
  });

  it('extracts app routes when present without matching git branch names', () => {
    const parts = splitWhereLinks('Open /needs-you in Hearth or feat/factory-phone-20261101.');
    expect(parts).toEqual([
      { text: 'Open ', href: null },
      { text: '/needs-you', href: '/needs-you' },
      { text: ' in Hearth or feat/factory-phone-20261101.', href: null }
    ]);
  });
});

describe('stepProgressLabel and yourMoveList', () => {
  it('formats step X of Y and returns null when no steps exist', () => {
    expect(stepProgressLabel(row({ steps: [] }))).toBeNull();
    expect(
      stepProgressLabel(
        row({
          steps: [
            { text: 'build it', owner: 'agent', done: true, done_at: '2026-11-01' },
            { text: 'deploy it', owner: 'agent', done: false, done_at: '' },
            { text: 'tap Screen', owner: 'you', done: false, done_at: '' },
            { text: 'tap Answer', owner: 'you', done: false, done_at: '' },
            { text: 'tap Works', owner: 'you', done: false, done_at: '' },
            { text: 'keep warm', owner: 'agent', done: false, done_at: '' }
          ]
        })
      )
    ).toBe('step 2 of 6');
  });

  it('collects undone steps owned by you, newest feature first', () => {
    const rows = [
      row({
        id: 'older-feat',
        name: 'Older Feature',
        area: 'LifeOS app',
        last_used: '2026-10-31',
        steps: [
          { text: 'agent built', owner: 'agent', done: true },
          { text: 'you step Done', owner: 'you', done: true, done_at: '2026-11-01' },
          { text: 'older you step', owner: 'you', done: false }
        ]
      }),
      row({
        id: 'newer-feat',
        name: 'Newer Feature',
        area: 'Tank',
        completed_at: '2026-11-02 06:00',
        steps: [
          { text: 'agent pending', owner: 'agent', done: false },
          { text: 'first you step', owner: 'you', done: false },
          { text: 'second you step', owner: 'you', done: false }
        ]
      })
    ];
    expect(yourMoveList(rows)).toEqual([
      {
        featureId: 'newer-feat',
        featureName: 'Newer Feature',
        area: 'Tank',
        stepIndex: 1,
        text: 'first you step',
        stepSummary: 'step 1 of 3',
        where: '',
        how: '',
        cmd: ''
      },
      {
        featureId: 'newer-feat',
        featureName: 'Newer Feature',
        area: 'Tank',
        stepIndex: 2,
        text: 'second you step',
        stepSummary: 'step 1 of 3',
        where: '',
        how: '',
        cmd: ''
      },
      {
        featureId: 'older-feat',
        featureName: 'Older Feature',
        area: 'LifeOS app',
        stepIndex: 2,
        text: 'older you step',
        stepSummary: 'step 3 of 3',
        where: '',
        how: '',
        cmd: ''
      }
    ]);
  });

  it('writes today as zero-padded YYYY-MM-DD', () => {
    expect(todayIso(new Date(2026, 0, 5))).toBe('2026-01-05');
    expect(todayIso(new Date(2026, 10, 15))).toBe('2026-11-15');
    expect(todayIso()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('maps app screen names to routes, else null', () => {
    expect(screenRouteFor('Needs You > session card')).toBe('/needs-you');
    expect(screenRouteFor('Hearth app > Settings')).toBe('/settings');
    expect(screenRouteFor('Builds page')).toBe('/builds');
    expect(screenRouteFor('Today')).toBe('/');
    expect(screenRouteFor('Termius on host')).toBeNull();
    expect(screenRouteFor('Host panel > Cron jobs')).toBeNull();
    expect(screenRouteFor('Glasses')).toBeNull();
    expect(screenRouteFor('This chat')).toBeNull();
    expect(screenRouteFor('')).toBeNull();
  });

  it('carries where, how, and cmd, trimmed, with empty defaults', () => {
    const rows = [
      row({
        id: 'cmd-feat',
        steps: [
          {
            text: 'restart the link',
            owner: 'you',
            done: false,
            where: '  Termius on host  ',
            how: 'Run the wrapper.',
            cmd: 'lo-restart'
          },
          { text: 'plain step', owner: 'you', done: false }
        ]
      })
    ];
    expect(yourMoveList(rows)).toEqual([
      {
        featureId: 'cmd-feat',
        featureName: rows[0].name,
        area: rows[0].area,
        stepIndex: 0,
        text: 'restart the link',
        stepSummary: 'step 1 of 2',
        where: 'Termius on host',
        how: 'Run the wrapper.',
        cmd: 'lo-restart'
      },
      {
        featureId: 'cmd-feat',
        featureName: rows[0].name,
        area: rows[0].area,
        stepIndex: 1,
        text: 'plain step',
        stepSummary: 'step 1 of 2',
        where: '',
        how: '',
        cmd: ''
      }
    ]);
  });
});
