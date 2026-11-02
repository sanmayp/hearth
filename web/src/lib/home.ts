/**
 * Home (Today tab): greeting, day ring, bars, rows, hero snooze.
 * Counts come from real lists only. Pure logic plus tiny storage.
 */

import { todayIso, type FeatureRow, type YourMoveItem } from './features';
import type { TodayView } from './today';

const DONE_KEY = 'hearth:done-log';
const SNOOZE_KEY = 'hearth:hero-snooze';

/** Morning before 12, afternoon before 18, evening after. */
export function greeting(hour: number): string {
  if (hour < 12) return 'Good morning';
  if (hour < 18) return 'Good afternoon';
  return 'Good evening';
}

/** "Sunday, November 2" in local time. */
export function dateLine(now = new Date()): string {
  return now.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' });
}

function storage(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

/** Completions this device recorded, id to date. Junk reads as empty. */
export function loadDoneLog(): Record<string, string> {
  try {
    const raw = storage()?.getItem(DONE_KEY);
    if (!raw) return {};
    const val: unknown = JSON.parse(raw);
    if (!val || typeof val !== 'object' || Array.isArray(val)) return {};
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(val as Record<string, unknown>)) {
      if (typeof v === 'string') out[k] = v;
    }
    return out;
  } catch {
    return {};
  }
}

/** Record one completion. Old dates drop out, so the log stays small. */
export function recordDone(key: string, today = todayIso()): void {
  const store = storage();
  if (!store) return;
  const log = loadDoneLog();
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(log)) {
    if (v === today) out[k] = v;
  }
  out[key] = today;
  try {
    store.setItem(DONE_KEY, JSON.stringify(out));
  } catch {
    /* device full: the ring just misses one */
  }
}

/** Completions stamped today. */
export function doneToday(log: Record<string, string>, today: string): number {
  return Object.values(log).filter((d) => d === today).length;
}

/** True when the owner tapped Later today. */
export function heroSnoozed(today = todayIso()): boolean {
  try {
    return storage()?.getItem(SNOOZE_KEY) === today;
  } catch {
    return false;
  }
}

/** Hide the hero until tomorrow. */
export function snoozeHero(today = todayIso()): void {
  try {
    storage()?.setItem(SNOOZE_KEY, today);
  } catch {
    /* snooze just fails quiet */
  }
}

/** True when a due stamp sits in the past. Null stamps never stall. */
export function isStalled(at: string | null, nowMs = Date.now()): boolean {
  if (!at) return false;
  const t = new Date(at).getTime();
  return !Number.isNaN(t) && t < nowMs;
}

/** "H:MM" local time for a due stamp. Empty when unreadable. */
export function dueTime(at: string | null): string {
  if (!at) return '';
  const d = new Date(at);
  if (Number.isNaN(d.getTime())) return '';
  return `${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`;
}

export type DayCounts = {
  /** Done today across steps, lists, and health. */
  done: number;
  /** Done plus open across the same lists. */
  total: number;
  /** Due reminders past their time. */
  stalled: number;
  /** Open items: total minus done. */
  waiting: number;
};

/**
 * Ring and bar counts. Open lists come from the server. Done
 * counts done-today steps, the health flag, and device completions.
 */
export function dayCounts(
  view: TodayView,
  openSteps: number,
  stepsDoneToday: number,
  logToday: number,
  nowMs = Date.now()
): DayCounts {
  const healthOpen = view.health_checkin_done ? 0 : 1;
  const healthDone = view.health_checkin_done ? 1 : 0;
  const done = stepsDoneToday + logToday + healthDone;
  const total = done + view.reminders_due.length + view.tasks_due.length + openSteps + healthOpen;
  const stalled = view.reminders_due.filter((r) => isStalled(r.at, nowMs)).length;
  return { done, total, stalled, waiting: Math.max(0, total - done) };
}

/** Ring sweep, 0 to 1. Empty day reads as 0, never NaN. */
export function ringFrac(done: number, total: number): number {
  if (total <= 0) return 0;
  return Math.min(1, Math.max(0, done / total));
}

/** Stroke offset for the r=88 ring (dash 552.9). */
export function ringOffset(frac: number): number {
  return 552.9 * (1 - Math.min(1, Math.max(0, frac)));
}

/** Bar width share of the biggest bar. All zero reads as 0. */
export function barShare(n: number, max: number): number {
  if (max <= 0) return 0;
  return Math.min(100, Math.max(0, (n / max) * 100));
}

export type HomeRowKind = 'reminder' | 'task' | 'step' | 'health';

export type HomeRow = {
  key: string;
  kind: HomeRowKind;
  title: string;
  sub: string;
  tag: string;
  done: boolean;
  /** Step rows carry the move item for the shared step card. */
  move: YourMoveItem | null;
  /** Feature id plus step index for the step toggle call. */
  featureId: string;
  stepIndex: number;
};

/**
 * Mixed rows: due reminders, due tasks, health check-in, open
 * steps, then steps done today. Done rows drop when hideDone.
 */
export function buildRows(
  view: TodayView,
  features: FeatureRow[],
  moves: YourMoveItem[],
  todayStr: string,
  hideDone: boolean
): HomeRow[] {
  const rows: HomeRow[] = [];
  const push = (r: HomeRow) => {
    if (!hideDone || !r.done) rows.push(r);
  };
  for (const r of view.reminders_due) {
    const t = dueTime(r.at);
    push({
      key: `reminder:${r.id}`,
      kind: 'reminder',
      title: r.title,
      sub: t ? `Due ${t}.` : '',
      tag: '',
      done: false,
      move: null,
      featureId: '',
      stepIndex: -1
    });
  }
  for (const t of view.tasks_due) {
    push({
      key: `task:${t.id}`,
      kind: 'task',
      title: t.title,
      sub: '',
      tag: '',
      done: false,
      move: null,
      featureId: '',
      stepIndex: -1
    });
  }
  if (!view.health_checkin_done) {
    push({
      key: 'health:checkin',
      kind: 'health',
      title: 'Check in with your health.',
      sub: '',
      tag: 'health',
      done: false,
      move: null,
      featureId: '',
      stepIndex: -1
    });
  }
  for (const m of moves) {
    push({
      key: `step:${m.featureId}:${m.stepIndex}`,
      kind: 'step',
      title: m.text,
      sub: m.featureName,
      tag: 'dev',
      done: false,
      move: m,
      featureId: m.featureId,
      stepIndex: m.stepIndex
    });
  }
  for (const f of features) {
    (f.steps ?? []).forEach((s, stepIndex) => {
      if (s.owner !== 'you' || !s.done || (s.done_at ?? '') !== todayStr) return;
      const move: YourMoveItem = {
        featureId: f.id,
        featureName: f.name,
        area: f.area,
        stepIndex,
        text: s.text,
        stepSummary: null,
        where: (s.where ?? '').trim(),
        how: (s.how ?? '').trim(),
        cmd: (s.cmd ?? '').trim()
      };
      push({
        key: `step:${f.id}:${stepIndex}`,
        kind: 'step',
        title: s.text,
        sub: f.name,
        tag: 'dev',
        done: true,
        move,
        featureId: f.id,
        stepIndex
      });
    });
  }
  return rows;
}

/** Open you-steps plus steps done on the given date. */
export function stepCounts(
  moves: YourMoveItem[],
  features: FeatureRow[],
  todayStr: string
): { open: number; doneToday: number } {
  let doneToday = 0;
  for (const f of features) {
    for (const s of f.steps ?? []) {
      if (s.owner === 'you' && s.done && (s.done_at ?? '') === todayStr) doneToday += 1;
    }
  }
  return { open: moves.length, doneToday };
}
