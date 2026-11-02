/** Needs You mapping tests. Fixtures copy the real watch-log shape. */
import { describe, expect, it } from 'vitest';
import {
  cardFor,
  countFeed,
  digestLine,
  digestSegments,
  laneHref,
  pulseNode,
  quiet24h,
  sortThreads,
  stateAge,
  stripNodes,
  demoNeedsYou,
  type NeedThread
} from './needs-you';

function row(over: Partial<NeedThread>): NeedThread {
  return {
    name: 'lifeos',
    pane: 'w1:p19',
    state: 'idle',
    at: '2026-11-01 06:00',
    changed_at: '2026-11-01 06:00',
    alerts: [],
    question: null,
    ...over
  };
}

describe('cardFor', () => {
  it('maps a blocked thread with a prompt to need', () => {
    const t = row({
      state: 'blocked',
      alerts: ['Waiting on approval'],
      question: 'Shows an approval prompt. Needs the owner answer to continue.'
    });
    expect(cardFor(t)).toBe('need');
  });

  it('maps idle over 30 min to calm waiting', () => {
    expect(cardFor(row({ alerts: ['Idle over 30 min'] }))).toBe('waiting');
  });

  it('maps fresh idle to waiting too', () => {
    expect(cardFor(row({ state: 'idle' }))).toBe('waiting');
  });

  it('maps working to building', () => {
    expect(cardFor(row({ state: 'working' }))).toBe('building');
  });

  it('maps done to ready', () => {
    expect(cardFor(row({ state: 'done' }))).toBe('ready');
  });

  it('maps blocked with no prompt to paused', () => {
    expect(cardFor(row({ state: 'blocked' }))).toBe('paused');
  });

  it('maps a gone session to failed', () => {
    const t = row({ state: 'gone', at: null, changed_at: null, alerts: ['Session gone'] });
    expect(cardFor(t)).toBe('failed');
  });
});

describe('stateAge', () => {
  const now = new Date(2026, 10, 1, 8, 0);

  it('names minutes under one hour', () => {
    expect(stateAge('2026-11-01 07:48', now)).toBe('12 min');
  });

  it('names hours under one day', () => {
    expect(stateAge('2026-11-01 05:00', now)).toBe('3 h');
  });

  it('names days with correct plural', () => {
    expect(stateAge('2026-10-30 08:00', now)).toBe('2 days');
    expect(stateAge('2026-10-31 08:00', now)).toBe('1 day');
  });

  it('names missing stamps unknown', () => {
    expect(stateAge(null, now)).toBe('unknown time');
    expect(stateAge('not a stamp', now)).toBe('unknown time');
  });
});

describe('quiet24h', () => {
  const now = new Date(2026, 9, 15, 8, 0);

  it('flags stamps a day old or older', () => {
    expect(quiet24h('2026-10-14 07:59', now)).toBe(true);
    expect(quiet24h('2026-10-14 08:00', now)).toBe(true);
    expect(quiet24h('2026-10-13 08:00', now)).toBe(true);
  });

  it('stays loud for fresh and missing stamps', () => {
    expect(quiet24h('2026-10-14 08:01', now)).toBe(false);
    expect(quiet24h('2026-10-15 07:00', now)).toBe(false);
    expect(quiet24h(null, now)).toBe(false);
    expect(quiet24h('junk', now)).toBe(false);
  });
});

describe('counts and digest', () => {
  it('counts the demo feed by kind', () => {
    expect(countFeed(demoNeedsYou)).toEqual({
      threads: 5,
      need: 1,
      waiting: 1,
      building: 1,
      ready: 1,
      paused: 1,
      unused: 1
    });
  });

  it('writes one digest line with real counts', () => {
    expect(digestLine(demoNeedsYou)).toBe(
      '1 wait on you. 1 build. 1 wait for tasks. 1 sit ready. 1 built wait unused.'
    );
  });

  it('links each count to its lane', () => {
    expect(laneHref('need')).toBe('/threads?lane=need');
    expect(laneHref('ready')).toBe('/threads?lane=review');
    expect(laneHref('paused')).toBe('/threads?lane=paused');
  });

  it('segments the digest with matching text and links', () => {
    const segs = digestSegments(demoNeedsYou);
    expect(segs.map((s) => `${s.n} ${s.text}`).join('. ') + '.').toBe(
      digestLine(demoNeedsYou)
    );
    expect(segs.map((s) => s.href)).toEqual([
      '/threads?lane=need',
      '/threads?lane=building',
      '/threads?lane=waiting',
      '/threads?lane=review',
      '#unused-check'
    ]);
  });
});

describe('strip and pulse', () => {
  it('orders nodes with state tones and real counts', () => {
    expect(stripNodes(demoNeedsYou)).toEqual([
      {
        id: 'need',
        label: 'Answer',
        n: 1,
        tone: 'var(--hue-need)',
        href: '/threads?lane=need',
        why: 'wait on your answer'
      },
      {
        id: 'building',
        label: 'Build',
        n: 1,
        tone: 'var(--ok)',
        href: '/threads?lane=building',
        why: 'build now'
      },
      {
        id: 'waiting',
        label: 'Wait',
        n: 1,
        tone: 'var(--paper-dim)',
        href: '/threads?lane=waiting',
        why: 'wait calm for tasks'
      },
      {
        id: 'ready',
        label: 'Review',
        n: 1,
        tone: 'var(--ok)',
        href: '/threads?lane=review',
        why: 'sit ready for review'
      },
      {
        id: 'paused',
        label: 'Pause',
        n: 1,
        tone: 'var(--bad)',
        href: '/threads?lane=paused',
        why: 'stay paused'
      }
    ]);
  });

  it('pulses the waiter when one waits', () => {
    expect(pulseNode(demoNeedsYou)).toBe('need');
  });

  it('sorts finished newest first after attention threads', () => {
    const feed = [
      row({ name: 'old-done', pane: 'w1:p1', state: 'done', changed_at: '2026-11-01 06:00' }),
      row({ name: 'ask', pane: 'w1:p2', state: 'blocked', question: 'q' }),
      row({ name: 'new-done', pane: 'w1:p3', state: 'done', changed_at: '2026-11-01 08:00' }),
      row({ name: 'idle', pane: 'w1:p4', state: 'idle', changed_at: '2026-11-01 07:00' })
    ];
    expect(sortThreads(feed).map((t) => t.name)).toEqual([
      'ask',
      'new-done',
      'idle',
      'old-done'
    ]);
  });

  it('ranks paused over waiting over ready over building', () => {
    const feed = (states: string[]) => ({
      ...demoNeedsYou,
      threads: states.map((state, i) => row({ name: `t${i}`, pane: `w1:p${i}`, state }))
    });
    expect(pulseNode(feed(['gone', 'idle', 'done', 'working']))).toBe('paused');
    expect(pulseNode(feed(['idle', 'done', 'working']))).toBe('waiting');
    expect(pulseNode(feed(['done', 'working']))).toBe('ready');
    expect(pulseNode(feed(['working']))).toBe('building');
    expect(pulseNode(feed([]))).toBeNull();
  });
});
