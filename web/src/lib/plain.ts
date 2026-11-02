/**
 * Plain words (ASD-STE100): one idea per sentence, short
 * sentences, simple verbs, one word per meaning. All screens
 * build user text from these templates. The test enforces the
 * word cap and the banned list on every template.
 */

/** Complex verbs. Never ship these. Use start, stop, send, check, wait. */
export const BANNED_WORDS = [
  'initiate',
  'initiates',
  'terminate',
  'terminates',
  'utilize',
  'utilizes',
  'utilise',
  'ascertain',
  'leverage',
  'leverages',
  'facilitate',
  'commence'
];

/** Max words for a step sentence. */
export const STEP_WORDS = 20;
/** Max words for a description sentence. */
export const DESC_WORDS = 25;

export function wordCount(s: string): number {
  return s.trim().split(/\s+/).filter(Boolean).length;
}

/**
 * Violations in one text: over-limit sentences plus banned
 * words. Empty means the text ships.
 */
export function checkPlain(text: string, limit = DESC_WORDS): string[] {
  const bad: string[] = [];
  for (const s of text.split(/[.!?]+/)) {
    if (wordCount(s) > limit) bad.push(`over ${limit} words: ${s.trim().slice(0, 40)}`);
  }
  const words = text.toLowerCase().split(/[^a-z]+/);
  for (const b of BANNED_WORDS) {
    if (words.includes(b)) bad.push(`banned word: ${b}`);
  }
  return bad;
}

/** "Prices loads daily price files. It is working. It waits on you." */
export function laneLine(name: string, job: string, state: string, rel: string): string {
  const head = job ? `${name} ${job}.` : `${name} does work.`;
  const mid = `It is ${state}.`;
  return rel ? `${head} ${mid} It ${rel}.` : `${head} ${mid}`;
}

/** "It waits on Bars." Lane names keep their own case. */
export function waitLine(who: string): string {
  return `It waits on ${who}.`;
}

/** "Answer 1 question." Units ride with every number. */
export function answerLine(n: number): string {
  return n === 1 ? 'Answer 1 question.' : `Answer ${n} questions.`;
}

/** Disabled Approve with no approval waiting. */
export function nothingToApprove(): string {
  return 'Nothing to approve.';
}

/** Disabled action with the bridge down. */
export function bridgeOff(): string {
  return 'Bridge is off. Check the key.';
}

/** Approve when the probe could not read the pane. */
export function approvalUnknown(): string {
  return 'Approval state is unknown. Open Screen to check.';
}

/** Disabled action with no link. */
export function noLink(): string {
  return 'No link. Check the link and retry.';
}

/** Stale worker warning. Mins counts minutes. */
export function staleLine(mins: number): string {
  return `No change for ${mins} min. Check the session.`;
}

/** Last lane result. Rows and errors count items. */
export function resultLine(rows: number, errors: number): string {
  return `${rows} rows. ${errors} errors.`;
}

/** Spend as the lane reports it. */
export function spendLine(spend: string): string {
  return `Spend: ${spend}.`;
}

/** Progress when no source reports it. */
export function progressUnknown(): string {
  return 'Progress is unknown.';
}

/** Last commit for a lane. Age reads like "12 min". */
export function commitLine(sha: string, age: string): string {
  return `Commit ${sha} from ${age} ago.`;
}

/** Status with time. Age reads like "12 min". */
export function sinceLine(doing: string, age: string): string {
  return `${doing} for ${age}.`;
}
