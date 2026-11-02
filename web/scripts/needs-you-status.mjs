#!/usr/bin/env node
// Builds the Needs You feed from the live Muse watch log.
// Writes web/static/needs-you.json. Runs on predev and prebuild.
// No deps, no network. Panes come from the lines themselves.
// A missing log bakes zero threads, never an error.
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { homedir } from 'os';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

const STATES = new Set(['working', 'idle', 'blocked', 'done', 'gone']);

// Split one cell into pane, state, alerts, plus an optional name.
// Old cells read `pane state`; new cells read `pane name state`.
function splitCell(words) {
  const [pane, w1, ...rest] = words;
  if (rest.length === 0) return { pane, state: w1, raw: [], name: null };
  if (STATES.has(w1) && !STATES.has(rest[0])) {
    return { pane, state: w1, raw: rest, name: null };
  }
  return { pane, state: rest[0], raw: rest.slice(1), name: w1 };
}

function daysFromCivil(y, m, d) {
  if (m <= 2) y -= 1;
  const era = Math.floor(y / 400);
  const yoe = y - era * 400;
  const mp = (m + 9) % 12;
  const doy = Math.floor((153 * mp + 2) / 5) + d - 1;
  const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
  return era * 146097 + doe - 719468;
}

// Minutes for a log stamp. Null when unreadable.
function stampMin(ts) {
  const m = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})/.exec(ts || '');
  if (!m) return null;
  const [y, mo, d, h, mi] = m.slice(1).map(Number);
  if (mo < 1 || mo > 12 || d < 1 || d > 31 || h > 23 || mi > 59) return null;
  return daysFromCivil(y, mo, d) * 1440 + h * 60 + mi;
}

// Time the current state run began, ignoring alert changes.
function stateSince(entries, state) {
  let first = entries[entries.length - 1].ts;
  for (let i = entries.length - 1; i >= 0; i--) {
    if (entries[i].state === state) first = entries[i].ts;
    else break;
  }
  return first;
}

const APPROVAL_Q = 'Shows an approval prompt. Needs the owner answer to continue.';
const MAX_EVENTS = 8;

function plainAlert(token) {
  if (token.startsWith('BLOCKED')) return 'Waiting on approval';
  if (token === 'GONE') return 'Session gone';
  if (token === 'IDLE>30min') return 'Idle over 30 min';
  if (token === 'SAME_SCREEN_WHILE_WORKING') return 'Screen unchanged while working';
  return token;
}

function parseLog(text) {
  let logAt = null;
  const byPane = new Map();
  const names = new Map();
  for (const line of String(text).split('\n')) {
    if (!line.startsWith('- ')) continue;
    const cut = line.indexOf(' | ');
    if (cut < 0) continue;
    const ts = line.slice(2, cut).trim();
    if (ts.length < 16) continue;
    logAt = ts;
    for (const cell of line.slice(cut + 3).split('|')) {
      const words = cell.trim().split(/\s+/).filter(Boolean);
      if (words.length < 2) continue;
      const { pane, state, raw, name } = splitCell(words);
      if (!byPane.has(pane)) byPane.set(pane, []);
      byPane.get(pane).push({ ts, state, raw });
      if (name) names.set(pane, name);
    }
  }
  return { byPane, names, logAt };
}

function changedSince(entries) {
  const last = entries[entries.length - 1];
  let first = last.ts;
  for (let i = entries.length - 1; i >= 0; i--) {
    const e = entries[i];
    if (e.state === last.state && JSON.stringify(e.raw) === JSON.stringify(last.raw)) {
      first = e.ts;
    } else {
      break;
    }
  }
  return first;
}

// One plain line per change. Real states only, no invented stages.
function eventText(name, state, raw) {
  if (raw.some((t) => t.startsWith('BLOCKED'))) return `${name} waits on approval.`;
  if (state === 'working' && raw.includes('SAME_SCREEN_WHILE_WORKING')) {
    return `${name} works. The screen stays still.`;
  }
  if (state === 'working') return `${name} starts building.`;
  if (state === 'done') return `${name} finishes. Ready for review.`;
  if (state === 'idle' && raw.includes('IDLE>30min')) return `${name} idles over 30 min.`;
  if (state === 'idle') return `${name} idles.`;
  if (state === 'blocked') return `${name} pauses. No prompt shows.`;
  return `${name} leaves. Session gone.`;
}

function eventTag(state, raw) {
  if (raw.some((t) => t.startsWith('BLOCKED'))) return 'NEED';
  if (state === 'working') return 'WORK';
  if (state === 'done') return 'DONE';
  if (raw.includes('IDLE>30min')) return 'STALL';
  if (state === 'blocked') return 'PAUSE';
  if (state === 'gone') return 'GONE';
  return 'IDLE';
}

// Newest changes first. Caps at MAX_EVENTS.
function buildEvents(byPane, names) {
  const events = [];
  const panes = [...byPane.keys()].sort();
  for (const pane of panes) {
    const entries = byPane.get(pane) || [];
    const name = names.get(pane) || pane;
    for (let i = 1; i < entries.length; i++) {
      const p = entries[i - 1];
      const e = entries[i];
      const same = e.state === p.state && JSON.stringify(e.raw) === JSON.stringify(p.raw);
      if (same) continue;
      events.push({ at: e.ts, tag: eventTag(e.state, e.raw), text: eventText(name, e.state, e.raw) });
    }
  }
  events.sort((a, b) => (a.at < b.at ? 1 : -1));
  return events.slice(0, MAX_EVENTS);
}

function buildFeed(text, unused) {
  const { byPane, names, logAt } = parseLog(text);
  const now = logAt ? stampMin(logAt) : null;
  const panes = [...byPane.keys()].sort();
  const threads = [];
  for (const pane of panes) {
    const entries = byPane.get(pane);
    const last = entries[entries.length - 1];
    const seen = stampMin(last.ts);
    if (now !== null && seen !== null && now - seen > 65) continue;
    if (last.state === 'gone') {
      const from = stampMin(stateSince(entries, 'gone'));
      if (now !== null && from !== null && now - from > 60) continue;
    }
    const name = names.get(pane) || pane;
    const blocked = last.state === 'blocked' && last.raw.some((t) => t.startsWith('BLOCKED'));
    threads.push({
      name,
      pane,
      state: last.state,
      at: last.ts,
      changed_at: changedSince(entries),
      alerts: last.raw.map(plainAlert),
      question: blocked ? APPROVAL_Q : null
    });
  }
  return { threads, unused, events: buildEvents(byPane, names), log_at: logAt };
}

let text = '';
try {
  const p = process.env.MUSE_WATCH_LOG || join(homedir(), 'hearth/MUSE-WATCH-LOG.md');
  text = readFileSync(p, 'utf8');
} catch {
  text = '';
}

let unused = [];
try {
  const raw = JSON.parse(readFileSync(join(root, 'web', 'data', 'built-unused.json'), 'utf8'));
  if (Array.isArray(raw.features)) unused = raw.features;
} catch {
  unused = [];
}

const feed = buildFeed(text, unused);
feed.generated_at = new Date().toISOString();
writeFileSync(join(root, 'web', 'static', 'needs-you.json'), JSON.stringify(feed, null, 2) + '\n');
console.log(`needs-you: threads=${feed.threads.length} unused=${unused.length} events=${feed.events.length}, log_at=${feed.log_at}`);
