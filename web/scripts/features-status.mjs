#!/usr/bin/env node
// Copies the features ledger into web/static/features.json for snapshot fallback.
// Runs on predev and prebuild. No deps, no network.
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { homedir } from 'os';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const srcPath = process.env.FEATURES_LEDGER || join(homedir(), 'hearth/FEATURES.json');

let data = { updated: null, features: [] };
try {
  const parsed = JSON.parse(readFileSync(srcPath, 'utf8'));
  if (parsed && Array.isArray(parsed.features)) {
    data = parsed;
  }
} catch {
  data = { updated: null, features: [] };
}

const out = {
  ...data,
  generated_at: new Date().toISOString()
};
writeFileSync(join(root, 'web', 'static', 'features.json'), JSON.stringify(out, null, 2) + '\n');
console.log(`features-status: ${out.features.length} features, updated=${out.updated}`);
