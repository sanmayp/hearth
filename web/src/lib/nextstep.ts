/**
 * Do-this-next: exactly ONE suggested next step for Home.
 * Pure logic only.
 *
 * Pick order: top open you-step (Features list order is the
 * module order; only Features owns steps), else first thread
 * with a question, else first wait/ready strip claim, else
 * first unopened built screen, else nothing.
 *
 * Step routing: reply-like text answers in Threads, a where
 * that names an app screen jumps to that screen, anything
 * else opens the Features page to read the full step.
 */

import { screenRouteFor, yourMoveList, type FeatureRow } from './features';
import { stripNodes, type NeedsYouFeed } from './needs-you';
import { isUsed, type ScreenUseMap } from './screenuse';

export type NextSuggestion = {
  /** Step text, question, claim, or screen name. */
  name: string;
  /** Feature, thread, or card context. */
  context: string;
  /** Screen, URL, or tap path. Empty when unknown. */
  where: string;
  /** One how line. Empty when unknown. */
  how: string;
  /** One shell command. Empty when the step needs no shell. */
  cmd: string;
  /** Full text for the Copy button. */
  copyText: string;
  /** Single CTA label. */
  ctaLabel: string;
  /** Single CTA target: route or same-page anchor. */
  ctaHref: string;
  /** True when the CTA also opens the screens checklist. */
  openChecklist: boolean;
};

/** Step text that means "answer the agent in Threads". */
const RESPOND_RE = /\b(send|reply|respond|answer)\b/i;

function joinLines(lines: string[]): string {
  return lines.filter((l) => l.trim().length > 0).join('\n');
}

/**
 * One suggestion, or null when Home holds no next step.
 * Rows carry the Features ledger; feed carries threads, strip,
 * and the built-unused list; useMap marks opened screens.
 */
export function pickNext(
  feed: NeedsYouFeed,
  rows: FeatureRow[],
  useMap: ScreenUseMap
): NextSuggestion | null {
  const steps = yourMoveList(rows);
  if (steps.length > 0) {
    const s = steps[0];
    const context = s.area ? `${s.featureName} · ${s.area}` : s.featureName;
    const copyText = joinLines([
      s.text,
      s.where ? `Where: ${s.where}` : '',
      s.how ? `How: ${s.how}` : '',
      s.cmd ? `Run: ${s.cmd}` : ''
    ]);
    if (RESPOND_RE.test(s.text)) {
      return {
        name: s.text,
        context,
        where: s.where || 'Threads',
        how: s.how,
        cmd: s.cmd,
        copyText,
        ctaLabel: 'Respond',
        ctaHref: '/threads',
        openChecklist: false
      };
    }
    const route = screenRouteFor(s.where);
    if (route) {
      return {
        name: s.text,
        context,
        where: s.where,
        how: s.how,
        cmd: s.cmd,
        copyText,
        ctaLabel: 'Go',
        ctaHref: route,
        openChecklist: false
      };
    }
    return {
      name: s.text,
      context,
      where: s.where,
      how: s.how,
      cmd: s.cmd,
      copyText,
      ctaLabel: 'See the step',
      ctaHref: '/features',
      openChecklist: false
    };
  }

  const need = feed.threads.find((t) => t.question);
  if (need?.question) {
    return {
      name: need.question,
      context: need.name,
      where: 'Threads',
      how: `Reply to ${need.name}.`,
      cmd: '',
      copyText: `${need.name}: ${need.question}`,
      ctaLabel: 'Respond',
      ctaHref: '/threads',
      openChecklist: false
    };
  }

  const claim = stripNodes(feed).find((n) => (n.id === 'waiting' || n.id === 'ready') && n.n > 0);
  if (claim) {
    const line = `${claim.n} ${claim.why}.`;
    return {
      name: claim.label,
      context: 'Your move',
      where: 'Needs You · Your move',
      how: line,
      cmd: '',
      copyText: line,
      ctaLabel: 'Read the claim',
      ctaHref: '#your-move',
      openChecklist: false
    };
  }

  const fresh = feed.unused.find((u) => !isUsed(useMap, u.route));
  if (fresh) {
    return {
      name: `Check ${fresh.name}.`,
      context: 'Screens to check',
      where: fresh.route,
      how: fresh.note,
      cmd: '',
      copyText: joinLines([`Check ${fresh.name}.`, fresh.note]),
      ctaLabel: 'Open checklist',
      ctaHref: '#unused-check',
      openChecklist: true
    };
  }

  return null;
}
