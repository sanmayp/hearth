#!/usr/bin/env node
// Parses the live Muse watch log into the thread board feed.
// Writes web/static/threads-status.json. Runs on predev and prebuild.
// No deps, no network. Mirrors modules/watch flow rules. Panes come
// from the lines themselves. A missing log bakes zero threads,
// never an error.
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

function plainAlert(token) {
  if (token.startsWith('BLOCKED')) return 'Waiting on approval';
  if (token === 'GONE') return 'Session gone';
  if (token === 'IDLE>30min') return 'Idle over 30 min';
  if (token === 'SAME_SCREEN_WHILE_WORKING') return 'Screen unchanged while working';
  return token;
}

function flowLine(state, question) {
  if (question) return ['need', 'Waits on the owner answer.'];
  if (state === 'working') return ['building', 'Builds in the background.'];
  if (state === 'done') return ['review', 'Ready for review.'];
  if (state === 'gone') return ['paused', 'Session gone.'];
  if (state === 'blocked') return ['paused', 'Pauses. No prompt shows.'];
  return ['waiting', 'Waits for the next task.'];
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

function buildFeed(text) {
  const { byPane, names, logAt } = parseLog(text);
  const now = logAt ? stampMin(logAt) : null;
  const panes = [...byPane.keys()].sort();
  const needsAnswer = [];
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
    const alerts = last.raw.map(plainAlert);
    const question =
      last.state === 'blocked' && last.raw.some((t) => t.startsWith('BLOCKED'))
        ? APPROVAL_Q
        : null;
    const [flow, line] = flowLine(last.state, question);
    if (flow === 'need' && question) {
      needsAnswer.push({ thread: name, question, recommendation: null });
    }
    threads.push({
      name,
      pane,
      state: last.state,
      flow,
      at: last.ts,
      changed_at: changedSince(entries),
      line,
      stage: null,
      question,
      recommendation: null
    });
  }
  return { threads, needs_answer: needsAnswer, log_at: logAt };
}

let text = '';
try {
  const p = process.env.MUSE_WATCH_LOG || join(homedir(), 'hearth/MUSE-WATCH-LOG.md');
  text = readFileSync(p, 'utf8');
} catch {
  text = '';
}

const feed = buildFeed(text);
feed.generated_at = new Date().toISOString();
writeFileSync(join(root, 'web', 'static', 'threads-status.json'), JSON.stringify(feed, null, 2) + '\n');
const counts = {};
for (const t of feed.threads) counts[t.flow] = (counts[t.flow] || 0) + 1;
console.log(`threads-status: ${JSON.stringify(counts)}, log_at=${feed.log_at}`);
