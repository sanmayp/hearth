/**
 * Dev flow: lane nodes, states, arcs, timeline. Real data only.
 * Unknowns stay unknown. Pure logic only.
 */

import { stateAge, type NeedThread } from './needs-you';

export type LaneMeta = {
  lane: string;
  at: string;
  note: string;
  rows: number | null;
  errors: number | null;
  pct: number | null;
  spend: string;
};

export type DevCommit = { sha: string; msg: string; at: string };

export type DevTests = { tests: number; failures: number; errors: number; at: string };

export type DevFeed = {
  lanes: LaneMeta[];
  lanes_note: string;
  commits: DevCommit[];
  commits_note: string;
  tests: DevTests | null;
  tests_note: string;
};

export const EMPTY_DEV_FEED: DevFeed = {
  lanes: [],
  lanes_note: '',
  commits: [],
  commits_note: '',
  tests: null,
  tests_note: ''
};

export type DevState =
  | 'working'
  | 'waiting'
  | 'blocked'
  | 'idle'
  | 'done';

export type NodeState = {
  key: DevState;
  /** Short label: Working, Needs answer, Blocked, Idle, Done. */
  label: string;
  /** Reason: exact question, alert, or state line. */
  reason: string;
  tone: string;
};

const TONE: Record<DevState, string> = {
  working: 'var(--ok)',
  waiting: 'var(--wait)',
  blocked: 'var(--bad)',
  idle: 'var(--paper-dim)',
  done: 'var(--ok)'
};

const LABEL: Record<DevState, string> = {
  working: 'Working',
  waiting: 'Needs answer',
  blocked: 'Blocked',
  idle: 'Idle',
  done: 'Done'
};

/** One state plus its reason. Questions win over raw states. */
export function nodeState(t: NeedThread): NodeState {
  if (t.question) {
    return { key: 'waiting', label: LABEL.waiting, reason: t.question, tone: TONE.waiting };
  }
  if (t.state === 'working') {
    return { key: 'working', label: LABEL.working, reason: 'It builds now.', tone: TONE.working };
  }
  if (t.state === 'blocked') {
    return {
      key: 'blocked',
      label: LABEL.blocked,
      reason: t.alerts[0] ?? 'No reason on record.',
      tone: TONE.blocked
    };
  }
  if (t.state === 'done') {
    return { key: 'done', label: LABEL.done, reason: 'It finished.', tone: TONE.done };
  }
  if (t.state === 'gone') {
    return {
      key: 'blocked',
      label: LABEL.blocked,
      reason: 'Session is gone.',
      tone: TONE.blocked
    };
  }
  return { key: 'idle', label: LABEL.idle, reason: 'It waits calm.', tone: TONE.idle };
}

/** Lane meta whose name matches the thread name or pane. */
export function matchLane(t: NeedThread, lanes: LaneMeta[]): LaneMeta | null {
  const hay = `${t.name} ${t.pane}`.toLowerCase();
  for (const l of lanes) {
    const n = l.lane.toLowerCase();
    if (n && (hay.includes(n) || n.includes(t.name.toLowerCase()))) return l;
  }
  return null;
}

/** Activity sentence. Alerts first, then screen, then lane note. */
export function activityLine(
  t: NeedThread,
  lane: LaneMeta | null,
  screenLast: string
): string {
  if (t.alerts[0]) return t.alerts[0];
  if (screenLast) return screenLast;
  if (lane?.note) return lane.note;
  return 'Unknown.';
}

/** True when a worker sat quiet for 15 min. Null stamps stay quiet. */
export function staleWorking(t: NeedThread, now = new Date()): boolean {
  if (t.state !== 'working') return false;
  const mins = minsSince(t.changed_at, now);
  return mins !== null && mins >= 15;
}

/** Minutes since the stamp. Null when unreadable. */
export function minsSince(changedAt: string | null, now = new Date()): number | null {
  if (!changedAt) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})/.exec(changedAt);
  if (!m) return null;
  const d = new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]);
  if (Number.isNaN(d.getTime())) return null;
  return Math.max(0, Math.floor((now.getTime() - d.getTime()) / 60000));
}

/** Arc label. Questions wait on you. Alerts can name a lane. */
export function arcLabel(t: NeedThread, lanes: LaneMeta[]): string {
  if (t.question) return 'waits on you';
  const a = (t.alerts[0] ?? '').toLowerCase();
  if (!a) return '';
  for (const l of lanes) {
    const n = l.lane.toLowerCase();
    if (n && a.includes(n)) return `waits on ${l.lane}`;
  }
  return '';
}

/** Newest commit that names the lane. Null when none does. */
export function lastCommitFor(lane: string, commits: DevCommit[]): DevCommit | null {
  const n = lane.toLowerCase();
  if (!n) return null;
  return commits.find((c) => c.msg.toLowerCase().includes(n)) ?? null;
}

/** Ribbon width grows with open waits. Six to fourteen units. */
export function ribbonWidth(openWaits: number): number {
  return 6 + 2 * Math.min(4, Math.max(0, openWaits));
}

export type LaneColor = 'green' | 'yellow' | 'red' | 'grey';

export const LANE_TONE: Record<LaneColor, string> = {
  green: 'var(--ok)',
  yellow: 'var(--wait)',
  red: 'var(--bad)',
  grey: 'var(--paper-dim)'
};

export type ColorInput = {
  state: DevState;
  /** Idle minutes, null when the stamp is missing. */
  idleMins: number | null;
  /** True when a worker sat quiet for 15 min. */
  stale: boolean;
  /** True when a bridge approval waits. */
  approval: boolean;
};

/**
 * Line and node color from lane state. Green works, yellow
 * waits, red is stuck or gone, grey rests. Amber never shows
 * status. Same source as the lane chips.
 */
export function laneColor(c: ColorInput): LaneColor {
  if (c.state === 'blocked') return 'red';
  if (c.stale) return 'red';
  if (c.state === 'waiting' || c.approval) return 'yellow';
  if (c.state === 'working') return 'green';
  if (c.state === 'done') return 'green';
  if (c.idleMins !== null && c.idleMins > 30) return 'yellow';
  return 'grey';
}

/** Dots run only on healthy flow. Stuck and held lanes sit still. */
export function hasFlow(c: ColorInput): boolean {
  return c.state === 'working' && !c.stale && !c.approval;
}

export type SlotItem = { id: string; state: DevState };

export type SlotPick = { shown: string[]; done: number; more: number };

/**
 * Ribbon slots: waiting lanes first and never hidden, then
 * blocked, then at most 6 working, then idle to fill 12. Done
 * lanes fold into the done count.
 */
export function selectSlots(items: SlotItem[]): SlotPick {
  const waiting = items.filter((i) => i.state === 'waiting');
  const blocked = items.filter((i) => i.state === 'blocked');
  const working = items.filter((i) => i.state === 'working').slice(0, 6);
  const idle = items.filter((i) => i.state === 'idle');
  const done = items.filter((i) => i.state === 'done').length;
  const shown = [...waiting, ...blocked, ...working];
  for (const i of idle) {
    if (shown.length >= 12) break;
    shown.push(i);
  }
  const live = items.length - done;
  return { shown: shown.map((s) => s.id), done, more: Math.max(0, live - shown.length) };
}

export type ChipCounts = { stalled: number; need: number; building: number };

/** Chip counts from threads plus bridge approvals. */
export function chipCounts(threads: NeedThread[], approvals: number): ChipCounts {
  let stalled = 0;
  let need = approvals;
  let building = 0;
  for (const t of threads) {
    const k = nodeState(t).key;
    if (k === 'blocked') stalled += 1;
    if (k === 'waiting') need += 1;
    if (k === 'working') building += 1;
  }
  return { stalled, need, building };
}

export type TimelineRow = {
  kind: 'commit' | 'tests';
  tone: string;
  title: string;
  sub: string;
  at: string;
};

/** Commits plus the newest test run, newest first. */
export function timelineRows(commits: DevCommit[], tests: DevTests | null): TimelineRow[] {
  const rows: TimelineRow[] = commits.map((c) => ({
    kind: 'commit' as const,
    tone: 'var(--ok)',
    title: `Commit ${c.sha.slice(0, 7)}: ${c.msg}`,
    sub: c.at,
    at: c.at
  }));
  if (tests) {
    const bad = tests.failures + tests.errors;
    rows.push({
      kind: 'tests',
      tone: bad > 0 ? 'var(--bad)' : 'var(--ok)',
      title: bad > 0 ? `Test run FAIL: ${tests.failures} fail, ${tests.errors} errors.` : `Test run PASS: ${tests.tests} tests.`,
      sub: tests.at || 'time unknown',
      at: tests.at
    });
  }
  rows.sort((a, b) => (a.at < b.at ? 1 : a.at > b.at ? -1 : 0));
  return rows;
}

/** "hh:mm" stamp for the last update line. */
export function stampHM(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0');
  return `${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** Age line for a lane node, or "unknown time". */
export function nodeAge(t: NeedThread): string {
  return stateAge(t.changed_at);
}

/**
 * Ribbon curve from a node slot to the YOU hexagon. Slots sit
 * in two columns in a 360-wide viewBox; endY marks the hexagon
 * top. Index 0 starts top left.
 */
export function ribbonPath(i: number, perCol: number, endY = 392): string {
  const col = i % 2 === 0 ? 0 : 1;
  const row = Math.floor(i / 2) % Math.max(1, perCol);
  const x = col === 0 ? 84 : 276;
  const y = 72 + row * 88;
  const mid = y + (endY - y) * 0.55;
  const ex = col === 0 ? 160 : 200;
  return `M${x} ${y} C ${x} ${mid.toFixed(0)}, ${ex} ${mid.toFixed(0)}, ${ex} ${endY}`;
}

/** Hexagon top for a row count. Four rows keep the mockup 392. */
export function hexTop(rows: number): number {
  return Math.max(4, rows) * 88 + 40;
}
