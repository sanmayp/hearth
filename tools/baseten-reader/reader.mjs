#!/usr/bin/env node
// Baseten reader demo. It summarizes one public text file.
// Live mode calls an OpenAI-compatible endpoint on Baseten.
// Mock mode prints a fixed summary. No key, no network.
// The key comes from env only. This file never prints it.

const fs = await import('node:fs');

const args = process.argv.slice(2);
const mock = args.includes('--mock');
const file = args.find((a) => !a.startsWith('--'));
if (!file) {
  console.error('Usage: reader.mjs [--mock] <textfile>');
  process.exit(2);
}
const text = fs.readFileSync(file, 'utf8').slice(0, 2000);

if (mock) {
  const first = text
    .split(/(?<=[.!?])\s+/)
    .filter((s) => s.trim().length > 0)
    .slice(0, 2)
    .join(' ');
  console.log('MOCK MODE. No network. No key.');
  console.log(first);
  process.exit(0);
}

const key = process.env.BASETEN_API_KEY;
const base = process.env.BASETEN_BASE_URL ?? 'https://inference.baseten.co/v1';
const model = process.env.BASETEN_MODEL;
if (!key || !model) {
  console.error('Set BASETEN_API_KEY and BASETEN_MODEL, or run with --mock.');
  process.exit(2);
}
const res = await fetch(`${base}/chat/completions`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${key}` },
  body: JSON.stringify({
    model,
    max_tokens: 200,
    messages: [
      {
        role: 'system',
        content: 'Summarize the user text in at most 3 short plain sentences.'
      },
      { role: 'user', content: text }
    ]
  })
});
if (!res.ok) {
  console.error(`Baseten call failed: HTTP ${res.status}.`);
  process.exit(1);
}
const data = await res.json();
console.log(data.choices?.[0]?.message?.content?.trim() ?? '(empty reply)');
