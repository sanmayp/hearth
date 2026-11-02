/** AI usage client. Reads batch meters. Zero when nothing ran. */
import { apiGet } from './api';

export type ProviderTotals = {
  calls: number;
  live: number;
  mock: number;
  tokens_in: number;
  tokens_out: number;
  p50_ms: number;
  p95_ms: number;
  errors: number;
};

export type UsageTotals = {
  calls: number;
  live: number;
  mock: number;
  tokens_in: number;
  tokens_out: number;
  errors: number;
};

export type UsageFeed = {
  providers: Record<string, ProviderTotals>;
  totals: UsageTotals;
};

export type ResultRow = {
  id: string;
  task: string;
  file: string;
  provider: string;
  model: string;
  output: string;
  tokens_in: number;
  tokens_out: number;
  latency_ms: number;
  status: string;
  mock: boolean;
  at_ms: number;
};

export type ResultsFeed = { rows: ResultRow[] };

/** Honest empty feed. Shown before any run. */
export const EMPTY_USAGE: UsageFeed = {
  providers: {},
  totals: { calls: 0, live: 0, mock: 0, tokens_in: 0, tokens_out: 0, errors: 0 }
};

/** Honest empty results. Shown before any run. */
export const EMPTY_RESULTS: ResultsFeed = { rows: [] };

/** Load usage totals. Throws when the link is down. */
export async function loadUsage(): Promise<UsageFeed> {
  const { data } = await apiGet<UsageFeed>('/ai/usage');
  return data;
}

/** Load result rows for one task. Throws when the link is down. */
export async function loadResults(task: string, limit: number): Promise<ResultRow[]> {
  const { data } = await apiGet<ResultsFeed>(
    `/ai/results?task=${encodeURIComponent(task)}&limit=${limit}`
  );
  return data.rows ?? [];
}

/** Bar width in percent. Zero max gives zero. */
export function barWidth(calls: number, max: number): number {
  if (max <= 0 || calls <= 0) return 0;
  return Math.min(100, Math.round((calls / max) * 100));
}

/** One-line totals. Plain words, real numbers. */
export function totalsLine(totals: UsageTotals): string {
  if (totals.calls === 0) return 'No runs yet.';
  const live = totals.live === 1 ? '1 live call' : `${totals.live} live calls`;
  const mock = totals.mock === 1 ? '1 mock call' : `${totals.mock} mock calls`;
  return `${live}, ${mock}.`;
}

/** Short lane label from a batch file path. */
export function laneLabel(file: string): string {
  const base = file.split('/').pop() ?? file;
  const stem = base.replace(/\.(txt|md)$/, '');
  return stem.replace(/^lane-/, '');
}
