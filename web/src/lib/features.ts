/** Features ledger: types, attention rules, owner confirm, proof, steps, your-move list, quiet 24h flag, area grouping, done sort, URL links, demo. Pure logic only. */

export type FeatureStage = 'idea' | 'building' | 'built' | 'tested' | 'deployed' | 'in_use';
export type OwnerConfirm = 'works' | 'broken';
export type StepOwner = 'you' | 'agent';

export type FeatureStep = {
  text: string;
  owner: StepOwner | string;
  done: boolean;
  done_at?: string;
  /** Exact screen or tab, e.g. "Settings > Bridge key". Empty when unset. */
  where?: string;
  /** One short how line. Empty when unset. */
  how?: string;
  /** One short terminal command, no flags. Empty when the step needs no shell. */
  cmd?: string;
};

export type FeatureRow = {
  id: string;
  name: string;
  area: string;
  stage: FeatureStage | string;
  where: string;
  branch?: string;
  commit?: string;
  pushed?: string;
  tested: string;
  deployed: string;
  last_used: string;
  next: string;
  sure?: string;
  completed_at?: string;
  owner_confirmed?: OwnerConfirm | string;
  owner_confirmed_at?: string;
  proof?: string;
  steps?: FeatureStep[];
};

export type FeaturesLedger = {
  _how?: string;
  updated: string | null;
  features: FeatureRow[];
  generated_at?: string;
};

export type WherePart = {
  text: string;
  href: string | null;
};

export type AreaGroup = {
  area: string;
  items: FeatureRow[];
};

export type YourMoveItem = {
  featureId: string;
  featureName: string;
  area: string;
  stepIndex: number;
  text: string;
  stepSummary: string | null;
  where: string;
  how: string;
  cmd: string;
};

export const STAGE_LABEL: Record<string, string> = {
  idea: 'Idea',
  building: 'Building',
  built: 'Built',
  tested: 'Tested',
  deployed: 'Deployed',
  in_use: 'In use'
};

/**
 * Stage chip colour:
 * idea gray, building blue, built amber, tested teal, deployed green, in_use bright green.
 */
export const STAGE_TONE: Record<string, string> = {
  idea: 'var(--paper-dim)',
  building: 'var(--hue-reminders)',
  built: 'var(--warn)',
  tested: 'var(--hue-business)',
  deployed: 'var(--ok)',
  in_use: '#6de36d'
};

export function stageLabel(stage: string): string {
  return STAGE_LABEL[stage] ?? stage;
}

export function stageTone(stage: string): string {
  return STAGE_TONE[stage] ?? 'var(--paper-dim)';
}

/** Claimed stage label shown side by side with owner confirmation: "claimed: <stage>". */
export function claimedLabel(f: FeatureRow): string {
  return `claimed: ${f.stage}`;
}

/** Short MM-DD date from YYYY-MM-DD or YYYY-MM-DD HH:MM. */
export function shortMonthDay(stamp?: string | null): string {
  const s = (stamp ?? '').trim();
  const m = /^(\d{4})-(\d{2}-\d{2})/.exec(s);
  if (m) return m[2];
  return s;
}

/**
 * Owner confirmation badge text shown side by side with claimed stage:
 * e.g. "you: works 11-02", "you: broken 11-02", or "you: unconfirmed".
 */
export function ownerConfirmLabel(f: FeatureRow): string {
  const c = (f.owner_confirmed ?? '').trim().toLowerCase();
  const md = shortMonthDay(f.owner_confirmed_at);
  if (c === 'works' || c === 'broken') {
    return md ? `you: ${c} ${md}` : `you: ${c}`;
  }
  return 'you: unconfirmed';
}

export function ownerConfirmTone(f: FeatureRow): string {
  const c = (f.owner_confirmed ?? '').trim().toLowerCase();
  if (c === 'works') return 'var(--ok)';
  if (c === 'broken') return 'var(--bad)';
  return 'var(--paper-dim)';
}

/**
 * Proof text per row (what was tested, by whom, when).
 * Returns trimmed proof or null when empty so the UI shows "no proof" in amber.
 */
export function proofText(f: FeatureRow): string | null {
  const p = (f.proof ?? '').trim();
  return p.length > 0 ? p : null;
}

/**
 * Step progress label per feature row: "step 2 of 6" (current active step, or "step N of N" when all done).
 * Returns null when the feature has no steps.
 */
export function stepProgressLabel(f: FeatureRow): string | null {
  const steps = f.steps ?? [];
  if (steps.length === 0) return null;
  const total = steps.length;
  const firstOpen = steps.findIndex((s) => !s.done);
  const current = firstOpen >= 0 ? firstOpen + 1 : total;
  return `step ${current} of ${total}`;
}

/**
 * Best recency timestamp for ordering features newest first.
 */
function featureRecencyKey(f: FeatureRow): string {
  const c = (f.completed_at ?? '').trim();
  const o = (f.owner_confirmed_at ?? '').trim();
  const u = (f.last_used ?? '').trim();
  const validU = u && u !== 'never' && u !== 'unknown' ? u : '';
  return [c, o, validU].sort().reverse()[0] ?? '';
}

/**
 * "Your move" list: only undone steps owned by "you", newest feature first.
 * Features with newer timestamps come first; within equal/empty timestamps, later ledger rows (newest added features) come first.
 */
/** App screen names to routes. First match wins. */
const SCREEN_ROUTES: [string, string][] = [
  ['needs you', '/needs-you'],
  ['threads', '/threads'],
  ['features', '/features'],
  ['settings', '/settings'],
  ['reminders', '/reminders'],
  ['tasks', '/tasks'],
  ['health', '/health'],
  ['inventory', '/inventory'],
  ['items', '/inventory'],
  ['business', '/business'],
  ['factory', '/factory'],
  ['builds', '/builds'],
  ['today', '/']
];

/** Today as YYYY-MM-DD on the device clock. Ledger stamps use this. */
export function todayIso(now = new Date()): string {
  const p = (n: number): string => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}`;
}

/**
 * App route for a step "where", or null when it names no app
 * screen (Termius, hosting panel, glasses, chat). The Go button shows
 * only when a route comes back.
 */
export function screenRouteFor(where: string): string | null {
  const w = where.toLowerCase();
  for (const [name, route] of SCREEN_ROUTES) {
    if (w.includes(name)) return route;
  }
  return null;
}

export function yourMoveList(rows: FeatureRow[]): YourMoveItem[] {
  const indexed = rows.map((f, idx) => ({ f, idx, key: featureRecencyKey(f) }));
  indexed.sort((a, b) => {
    if (a.key && b.key && a.key !== b.key) return b.key.localeCompare(a.key);
    if (a.key && !b.key) return -1;
    if (!a.key && b.key) return 1;
    return b.idx - a.idx;
  });
  const out: YourMoveItem[] = [];
  for (const { f } of indexed) {
    const steps = f.steps ?? [];
    const summary = stepProgressLabel(f);
    steps.forEach((s, stepIndex) => {
      const owner = (s.owner || 'you').trim().toLowerCase();
      if (owner === 'you' && !s.done) {
        out.push({
          featureId: f.id,
          featureName: f.name,
          area: f.area,
          stepIndex,
          text: s.text,
          stepSummary: summary,
          where: (s.where ?? '').trim(),
          how: (s.how ?? '').trim(),
          cmd: (s.cmd ?? '').trim()
        });
      }
    });
  }
  return out;
}

/**
 * Parse a date or timestamp ("YYYY-MM-DD" or "YYYY-MM-DD HH:MM") into a local Date.
 */
export function parseUsedStamp(stamp?: string | null): Date | null {
  const s = (stamp ?? '').trim();
  if (!s) return null;
  const dt = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})/.exec(s);
  if (dt) {
    const d = new Date(+dt[1], +dt[2] - 1, +dt[3], +dt[4], +dt[5]);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  const dOnly = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
  if (dOnly) {
    const d = new Date(+dOnly[1], +dOnly[2] - 1, +dOnly[3], 0, 0);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  return null;
}

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * True when last_used is older than 24 h (or never/unknown).
 */
export function isQuiet24h(f: FeatureRow, now = new Date()): boolean {
  const raw = (f.last_used ?? '').trim().toLowerCase();
  if (!raw || raw === 'never' || raw === 'unknown') return true;
  const usedAt = parseUsedStamp(f.last_used);
  if (!usedAt) return true;
  return now.getTime() - usedAt.getTime() > DAY_MS;
}

/**
 * True when a feature counts as Done:
 * A feature counts as done ONLY when the owner confirms ("works").
 */
export function isDone(f: FeatureRow): boolean {
  return (f.owner_confirmed ?? '').trim().toLowerCase() === 'works';
}

/**
 * True when a feature needs attention:
 * owner marked it broken, OR stage is built/tested/deployed/in_use and not yet owner-confirmed "works".
 */
export function needsAttention(f: FeatureRow): boolean {
  const conf = (f.owner_confirmed ?? '').trim().toLowerCase();
  if (conf === 'broken') return true;
  if (conf === 'works') return false;
  if (f.stage === 'built' || f.stage === 'tested') return true;
  if (f.stage === 'deployed' || f.stage === 'in_use') return true;
  return false;
}

/**
 * Sort done items newest first by completed_at (or owner_confirmed_at) descending.
 * Rows without completed_at go last.
 */
export function sortDoneNewestFirst<
  T extends { completed_at?: string | null; owner_confirmed_at?: string | null }
>(rows: T[]): T[] {
  return [...rows].sort((a, b) => {
    const ca = (a.completed_at ?? '').trim();
    const cb = (b.completed_at ?? '').trim();
    if (ca && cb) {
      if (ca === cb) return 0;
      return cb.localeCompare(ca);
    }
    if (ca && !cb) return -1;
    if (!ca && cb) return 1;
    const oa = (a.owner_confirmed_at ?? '').trim();
    const ob = (b.owner_confirmed_at ?? '').trim();
    if (oa && ob && oa !== ob) return ob.localeCompare(oa);
    return 0;
  });
}

/** Attention rows in ledger order. */
export function attentionList(rows: FeatureRow[]): FeatureRow[] {
  return rows.filter(needsAttention);
}

/** In-progress rows (not done). */
export function inProgressList(rows: FeatureRow[]): FeatureRow[] {
  return rows.filter((r) => !isDone(r));
}

/** Done rows (owner confirmed works) sorted newest completed first; rows without completed_at go last. */
export function doneList(rows: FeatureRow[]): FeatureRow[] {
  return sortDoneNewestFirst(rows.filter(isDone));
}

/**
 * Group features by area, preserving area encounter order.
 */
export function groupByArea(rows: FeatureRow[]): AreaGroup[] {
  const map = new Map<string, FeatureRow[]>();
  for (const r of rows) {
    const area = (r.area || 'Other').trim() || 'Other';
    const list = map.get(area);
    if (list) list.push(r);
    else map.set(area, [r]);
  }
  return Array.from(map.entries()).map(([area, items]) => ({ area, items }));
}

/**
 * Split a "Where it runs" text into plain text and tappable links (https://, http://, or /route).
 */
export function splitWhereLinks(where: string): WherePart[] {
  if (!where) return [{ text: 'Not specified.', href: null }];
  const re =
    /(https?:\/\/[^\s),]+)|(?<![\w/-])(\/(?:needs-you|threads|builds|factory|tasks|reminders|health|inventory|business|settings|features))(?!\S)/g;
  const parts: WherePart[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(where)) !== null) {
    let match = m[0];
    while (/[.;:!?]$/.test(match)) {
      match = match.slice(0, -1);
    }
    if (m.index > last) {
      parts.push({ text: where.slice(last, m.index), href: null });
    }
    parts.push({ text: match, href: match });
    last = m.index + match.length;
    re.lastIndex = last;
  }
  if (last < where.length) {
    parts.push({ text: where.slice(last), href: null });
  }
  return parts.length > 0 ? parts : [{ text: where, href: null }];
}

/** Sample ledger. Shows when both API and snapshot stay quiet. Never real data. */
export const demoFeatures: FeaturesLedger = {
  updated: '2026-11-02 06:15',
  features: [
    {
      id: 'demo-self-update',
      name: 'In-app update check',
      area: 'LifeOS app',
      stage: 'in_use',
      where: 'Signed manifest served by the API.',
      tested: 'Owner confirmed on S25 release 1004 (2026-11-02)',
      deployed: 'Releases 1002-1004 signed',
      last_used: '2026-11-02',
      next: 'none',
      completed_at: '2026-11-01 21:50',
      owner_confirmed: 'works',
      owner_confirmed_at: '2026-11-02',
      proof: 'Owner verified update prompt on S25 (2026-11-02 05:38)',
      steps: [{ text: 'done', owner: 'you', done: true, done_at: '2026-11-02' }]
    },
    {
      id: 'demo-agent-bridge',
      name: 'Agent bridge (control sessions from phone)',
      area: 'Hearth / host',
      stage: 'deployed',
      where: 'Runs on the host 127.0.0.1:8449.',
      tested: 'Fake-herdr tests (2026-11-01)',
      deployed: 'Loopback and tailnet 8449',
      last_used: '2026-11-02',
      next: 'Owner presses Screen on one session to confirm',
      completed_at: '2026-11-01 22:50',
      proof: '',
      steps: [
        {
          text: 'tap Screen',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Needs You > session card',
          how: 'Tap Screen and read the output.'
        },
        {
          text: 'tap Answer with confirm code',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Needs You > session card',
          how: 'Type the line, tap Preview, type the code.'
        },
        {
          text: 'restart the phone link',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Termius on host',
          how: 'Run the restart wrapper.',
          cmd: 'lo-restart'
        }
      ]
    },
    {
      id: 'demo-tank-gallons',
      name: 'Tank gallons, days left, order-status card',
      area: 'Tank',
      stage: 'tested',
      where: 'Branch feat/tank-gallons-20261101; not on live page.',
      tested: '78 of 78 tests (2026-11-01)',
      deployed: 'not deployed',
      last_used: 'never',
      next: 'Owner reviews screenshots then merge',
      completed_at: '',
      proof: '78 unit tests + 26 screenshots by Muse (2026-11-01)',
      steps: [
        {
          text: 'look at screenshots',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Builds page',
          how: 'Open the latest shots.'
        },
        {
          text: 'enroll phone with code',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Hearth app > Settings',
          how: 'Type the shown code.'
        },
        {
          text: 'say merge and deploy',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'This chat',
          how: 'Say the words.'
        }
      ]
    },
    {
      id: 'demo-glasses',
      name: 'Meta glasses voice-note companion app',
      area: 'Meta glasses',
      stage: 'built',
      where: 'APK on tailnet link.',
      tested: '37 unit tests pass; NOT tested on glasses',
      deployed: 'APK on tailnet link, not installed yet',
      last_used: 'never',
      next: 'Owner pairs glasses and installs',
      completed_at: '',
      proof: '',
      steps: [
        {
          text: 'pair glasses in Developer Mode',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Meta AI app > Devices',
          how: 'Follow the pairing flow.'
        },
        {
          text: 'install companion APK',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'This phone',
          how: 'Open the tailnet APK link.'
        },
        {
          text: 'record a voice note',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Glasses',
          how: 'Tap and speak.'
        },
        {
          text: 'test offline transcription',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Glasses',
          how: 'Turn airplane mode on first.'
        },
        {
          text: 'confirm on phone',
          owner: 'you',
          done: false,
          done_at: '',
          where: 'Hearth app',
          how: 'Check the note landed.'
        }
      ]
    }
  ]
};
