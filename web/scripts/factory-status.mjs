#!/usr/bin/env node
// Reads repo state and writes web/static/factory-status.json.
// Runs on predev and prebuild. No deps, no network.
import { readFileSync, readdirSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const reqDir = join(root, 'requirements');

function parseFront(path) {
  const text = readFileSync(path, 'utf8');
  const m = text.match(/^---\n([\s\S]*?)\n---/);
  const out = {};
  if (!m) return out;
  for (const line of m[1].split('\n')) {
    const i = line.indexOf(':');
    if (i < 0) continue;
    const key = line.slice(0, i).trim();
    let val = line.slice(i + 1).trim();
    if ((val.startsWith("'") && val.endsWith("'")) || (val.startsWith('"') && val.endsWith('"'))) {
      val = val.slice(1, -1);
    }
    out[key] = val;
  }
  return out;
}

function parseState() {
  let text = '';
  try {
    text = readFileSync(join(root, 'factory', 'STATE.md'), 'utf8');
  } catch {
    return {};
  }
  const sections = {};
  let cur = null;
  for (const line of text.split('\n')) {
    const h = line.match(/^## (.+)$/);
    if (h) {
      cur = h[1].trim();
      sections[cur] = [];
    } else if (cur && line.startsWith('- ')) {
      sections[cur].push(line.slice(2).trim());
    }
  }
  return sections;
}

function parseLedger() {
  let text = '';
  try {
    text = readFileSync(join(root, 'factory', 'ledger.jsonl'), 'utf8');
  } catch {
    return [];
  }
  return text
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => {
      try {
        return JSON.parse(l);
      } catch {
        return null;
      }
    })
    .filter(Boolean);
}

const reqs = readdirSync(reqDir)
  .filter((f) => f.startsWith('REQ-') && f.endsWith('.md'))
  .map((f) => {
    const fm = parseFront(join(reqDir, f));
    return { id: fm.id || f, title: fm.title || f, status: fm.status || 'unknown' };
  })
  .sort((a, b) => (a.id < b.id ? -1 : 1));

const status = {
  generated_at: new Date().toISOString(),
  reqs,
  state: parseState(),
  ledger: parseLedger()
};

writeFileSync(join(root, 'web', 'static', 'factory-status.json'), JSON.stringify(status, null, 2) + '\n');
console.log(`factory-status: ${reqs.length} reqs, ${status.ledger.length} ledger rows`);
