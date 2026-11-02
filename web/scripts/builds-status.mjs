#!/usr/bin/env node
// Reads the builds manifest plus live agent states.
// Writes web/static/builds-status.json. Runs on predev and prebuild.
// No deps, no network. Herdr runs best effort; demo names fill gaps.
import { execFileSync } from 'child_process';
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

// Same shape as web/src/lib/demo.ts demoAgents. Used when herdr stays quiet.
const demoAgents = [
  { id: 'a1', name: 'muse (lifeos)', state: 'working', job: 'Builds board plus voice.' },
  { id: 'a2', name: 'helper (site)', state: 'idle', job: 'Waits at prompt.' },
  { id: 'a3', name: 'muse (home)', state: 'idle', job: 'Waits at prompt.' },
  { id: 'a4', name: 'muse (tank)', state: 'working', job: 'Fixes test failures.' }
];

// Map raw herdr words to the four states the board knows.
function mapState(raw) {
  const s = String(raw || '').toLowerCase();
  if (/work|run|busy|active/.test(s)) return 'working';
  if (/block|error|fail|stuck/.test(s)) return 'blocked';
  if (/done|exit|finish|complete/.test(s)) return 'done';
  return 'idle';
}

// Parse `herdr agent list` text. Tolerant: JSON first, then table rows.
function parseAgents(text) {
  const clean = String(text || '').trim();
  if (!clean) return null;
  try {
    const j = JSON.parse(clean);
    const rows = Array.isArray(j) ? j : j.agents || j.data || [];
    if (!Array.isArray(rows) || rows.length === 0) return null;
    return rows.map((r, i) => ({
      id: String(r.id ?? r.agent ?? `a${i + 1}`),
      name: String(r.name ?? r.title ?? r.agent ?? `agent ${i + 1}`),
      state: mapState(r.state ?? r.status),
      job: String(r.job ?? r.task ?? r.prompt ?? '')
    }));
  } catch {
    /* not JSON: read it as a plain table */
  }
  const agents = [];
  for (const line of clean.split('\n')) {
    const t = line.trim();
    if (!t) continue;
    // Skip header, frame, and rule lines.
    if (/^[-+|│┌┐└┘├┤┬┴┼=\s]*$/.test(t)) continue;
    if (/^(id|name|agent|state|status)\b/i.test(t) && /[|│]/.test(t)) continue;
    const cells = t.split(/\s*[|│]\s*/).map((c) => c.trim()).filter(Boolean);
    if (cells.length < 2) continue;
    const n = agents.length;
    agents.push({
      id: cells[0].toLowerCase().startsWith('a') ? cells[0] : `a${n + 1}`,
      name: cells.length > 2 ? cells[1] : cells[0],
      state: mapState(cells.length > 2 ? cells[2] : cells[1]),
      job: cells.length > 3 ? cells.slice(3).join(' | ') : ''
    });
  }
  return agents.length > 0 ? agents : null;
}

function liveAgents() {
  try {
    const out = execFileSync('herdr', ['agent', 'list'], {
      timeout: 5000,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore']
    });
    return parseAgents(out) || demoAgents;
  } catch {
    return demoAgents;
  }
}

let builds = [];
try {
  builds = JSON.parse(readFileSync(join(root, 'factory', 'builds.json'), 'utf8'));
} catch {
  builds = [];
}

const status = {
  builds,
  agents: liveAgents(),
  at: new Date().toISOString()
};

writeFileSync(join(root, 'web', 'static', 'builds-status.json'), JSON.stringify(status, null, 2) + '\n');
console.log(`builds-status: ${builds.length} builds, ${status.agents.length} agents`);
