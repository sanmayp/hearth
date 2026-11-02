/** Needs You feed: types, card map, age labels, digest. Pure logic only. */

export type NeedKind = 'need' | 'waiting' | 'building' | 'ready' | 'paused' | 'failed';

export type NeedThread = {
  name: string;
  pane: string;
  state: string;
  at: string | null;
  changed_at: string | null;
  alerts: string[];
  question: string | null;
};

export type UnusedFeature = {
  id: string;
  name: string;
  route: string;
  shipped_at: string;
  note: string;
};

export type NeedEvent = { at: string; tag: string; text: string };

export type NeedsYouFeed = {
  threads: NeedThread[];
  unused: UnusedFeature[];
  events: NeedEvent[];
  log_at: string | null;
  generated_at?: string;
};

/** Map one thread row to one card kind. Real states only. */
export function cardFor(t: NeedThread): NeedKind {
  if (t.question) return 'need';
  if (t.state === 'working') return 'building';
  if (t.state === 'done') return 'ready';
  if (t.state === 'gone') return 'failed';
  if (t.state === 'blocked') return 'paused';
  return 'waiting';
}

/** Parse "YYYY-MM-DD HH:MM" log stamps as local time. Null when unreadable. */
export function stampToDate(stamp: string | null): Date | null {
  if (!stamp) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})/.exec(stamp);
  if (!m) return null;
  const d = new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** Plain age in state: "12 min", "3 h", "2 days", else "unknown time". */
export function stateAge(changedAt: string | null, now = new Date()): string {
  const d = stampToDate(changedAt);
  if (!d) return 'unknown time';
  const mins = Math.max(0, Math.floor((now.getTime() - d.getTime()) / 60000));
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins} min`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} h`;
  const days = Math.floor(hours / 24);
  return `${days} day${days === 1 ? '' : 's'}`;
}

/** True when the watcher saw no change for 24 h or more. Null stamps stay loud. */
export function quiet24h(changedAt: string | null, now = new Date()): boolean {
  const d = stampToDate(changedAt);
  if (!d) return false;
  return now.getTime() - d.getTime() >= 24 * 60 * 60 * 1000;
}

export type NeedCounts = {
  threads: number;
  need: number;
  waiting: number;
  building: number;
  ready: number;
  paused: number;
  unused: number;
};

/** Real counts for tiles, strip, and digest. */
export function countFeed(feed: NeedsYouFeed): NeedCounts {
  const c: NeedCounts = {
    threads: feed.threads.length,
    need: 0,
    waiting: 0,
    building: 0,
    ready: 0,
    paused: 0,
    unused: feed.unused.length
  };
  for (const t of feed.threads) {
    const k = cardFor(t);
    if (k === 'need') c.need += 1;
    else if (k === 'waiting') c.waiting += 1;
    else if (k === 'building') c.building += 1;
    else if (k === 'ready') c.ready += 1;
    else c.paused += 1;
  }
  return c;
}

export type StripNode = {
  id: string;
  label: string;
  n: number;
  tone: string;
  href: string;
  why: string;
};

/** Threads lane link for a thread kind. Ready opens the review lane. */
export function laneHref(kind: NeedKind): string {
  const lane = kind === 'ready' ? 'review' : kind;
  return `/threads?lane=${lane}`;
}

export type DigestSegment = {
  n: number;
  text: string;
  href: string;
};

/**
 * Digest as linked segments. Each number opens the exact list
 * it counts. Text matches digestLine, piece for piece.
 */
export function digestSegments(feed: NeedsYouFeed): DigestSegment[] {
  const c = countFeed(feed);
  return [
    { n: c.need, text: 'wait on you', href: laneHref('need') },
    { n: c.building, text: 'build', href: laneHref('building') },
    { n: c.waiting, text: 'wait for tasks', href: laneHref('waiting') },
    { n: c.ready, text: 'sit ready', href: laneHref('ready') },
    { n: c.unused, text: 'built wait unused', href: '#unused-check' }
  ];
}

/**
 * Strip nodes in comp order. Tones follow the strip palette:
 * yellow waits on the owner, green builds or sits done, red
 * pauses or fails, dim waits calm for the next task.
 */
export function stripNodes(feed: NeedsYouFeed): StripNode[] {
  const c = countFeed(feed);
  return [
    {
      id: 'need',
      label: 'Answer',
      n: c.need,
      tone: 'var(--hue-need)',
      href: laneHref('need'),
      why: 'wait on your answer'
    },
    {
      id: 'building',
      label: 'Build',
      n: c.building,
      tone: 'var(--ok)',
      href: laneHref('building'),
      why: 'build now'
    },
    {
      id: 'waiting',
      label: 'Wait',
      n: c.waiting,
      tone: 'var(--paper-dim)',
      href: laneHref('waiting'),
      why: 'wait calm for tasks'
    },
    {
      id: 'ready',
      label: 'Review',
      n: c.ready,
      tone: 'var(--ok)',
      href: laneHref('ready'),
      why: 'sit ready for review'
    },
    {
      id: 'paused',
      label: 'Pause',
      n: c.paused,
      tone: 'var(--bad)',
      href: laneHref('paused'),
      why: 'stay paused'
    }
  ];
}

/**
 * Card order for the Needs You screen. Threads that need the owner
 * keep feed order on top. Finished threads (waiting, ready) follow,
 * newest-completed first by state time, then log time.
 */
export function sortThreads(threads: NeedThread[]): NeedThread[] {
  const finished = (t: NeedThread) => {
    const k = cardFor(t);
    return k === 'waiting' || k === 'ready';
  };
  const key = (t: NeedThread) => t.changed_at || t.at || '';
  return [...threads]
    .map((t, i) => ({ t, i }))
    .sort((a, b) => {
      const fa = finished(a.t);
      const fb = finished(b.t);
      if (!fa && fb) return -1;
      if (fa && !fb) return 1;
      if (fa && fb) {
        const ka = key(a.t);
        const kb = key(b.t);
        if (ka !== kb) return ka < kb ? 1 : -1;
      }
      return a.i - b.i;
    })
    .map(({ t }) => t);
}

/** Top need wins the pulse: waiters first, builders last. */
export function pulseNode(feed: NeedsYouFeed): string | null {
  const c = countFeed(feed);
  if (c.need > 0) return 'need';
  if (c.paused > 0) return 'paused';
  if (c.waiting > 0) return 'waiting';
  if (c.ready > 0) return 'ready';
  if (c.building > 0) return 'building';
  return null;
}

/** One-line morning digest. Names counts, never advice. */
export function digestLine(feed: NeedsYouFeed): string {
  const c = countFeed(feed);
  const bits = [
    `${c.need} wait on you`,
    `${c.building} build`,
    `${c.waiting} wait for tasks`,
    `${c.ready} sit ready`,
    `${c.unused} built wait unused`
  ];
  return bits.join('. ') + '.';
}

/** Sample feed. Shows when the server stays quiet. Never real data. */
export const demoNeedsYou: NeedsYouFeed = {
  threads: [
    {
      name: 'lifeos',
      pane: 'w1:p19',
      state: 'working',
      at: null,
      changed_at: null,
      alerts: [],
      question: null
    },
    {
      name: 'muse-trade',
      pane: 'w1:p1X',
      state: 'blocked',
      at: null,
      changed_at: null,
      alerts: ['Waiting on approval'],
      question: 'Shows an approval prompt. Needs the owner answer to continue.'
    },
    {
      name: 'muse-rummy-engine',
      pane: 'w1:p1Z',
      state: 'idle',
      at: null,
      changed_at: null,
      alerts: ['Idle over 30 min'],
      question: null
    },
    {
      name: 'muse-station',
      pane: 'w1:p21',
      state: 'done',
      at: null,
      changed_at: null,
      alerts: [],
      question: null
    },
    {
      name: 'muse-supervisor',
      pane: 'w1:p22',
      state: 'gone',
      at: null,
      changed_at: null,
      alerts: ['Session gone'],
      question: null
    }
  ],
  unused: [
    {
      id: 'u1',
      name: 'Fake: sample build',
      route: '/builds',
      shipped_at: '2026-10-30',
      note: 'Sample row. No owner use yet.'
    }
  ],
  events: [
    { at: '2026-11-01 06:00', tag: 'NEED', text: 'muse-trade waits on approval.' },
    { at: '2026-11-01 05:55', tag: 'DONE', text: 'muse-station finishes. Ready for review.' }
  ],
  log_at: null
};
