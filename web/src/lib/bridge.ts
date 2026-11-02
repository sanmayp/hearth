/** Agent bridge client. Preview, confirm code, send. Key stays on device. */

import { API_BASE } from './config';
import { approvalUnknown, nothingToApprove } from './plain';

/** Default bridge port. Rocky serves the bridge here. */
export const BRIDGE_PORT_DEFAULT = '8449';
/** Max chars the Answer box sends. The bridge allows more. */
export const BRIDGE_TEXT_MAX = 1000;
/** Escape key marker. The bridge maps this byte to the Escape key. */
export const BRIDGE_ESC = '\u001b';

export type BridgePaneRow = {
  id: string;
  label: string;
  tab: string;
  kind: string;
  status: string;
};

export type BridgeOutput = {
  pane: string;
  lines: string[];
  count: number;
  redacted: boolean;
  approval_waiting: boolean;
  approval_prompt: string | null;
};

export type BridgePreview = {
  code: string;
  text: string;
  pane: string;
  expires_in: number;
  approval_waiting: boolean;
  approval_prompt: string | null;
};

export type BridgeError = { message: string; status: number; retryAfter: number | null };

/** Bridge key from device storage. Empty when the owner never pasted one. */
export function loadBridgeKey(): string {
  try {
    return globalThis.localStorage?.getItem('hearth:bridge-key') ?? '';
  } catch {
    return '';
  }
}

/** Keep the bridge key on this device only. Never send it but to the bridge. */
export function saveBridgeKey(v: string): void {
  try {
    globalThis.localStorage?.setItem('hearth:bridge-key', v);
  } catch {
    /* private mode: no store */
  }
}

/** Bridge port from device storage. Default 8449 on bad input. */
export function loadBridgePort(): string {
  try {
    const v = (globalThis.localStorage?.getItem('hearth:bridge-port') ?? '').trim();
    const n = Number(v);
    if (v && Number.isInteger(n) && n >= 1 && n <= 65535) return String(n);
  } catch {
    /* fall through to default */
  }
  return BRIDGE_PORT_DEFAULT;
}

/** Keep the bridge port on this device only. */
export function saveBridgePort(v: string): void {
  try {
    globalThis.localStorage?.setItem('hearth:bridge-port', v);
  } catch {
    /* private mode: no store */
  }
}

/**
 * Test seam: ?bridge= loopback base wins. Loopback only, never
 * remote, so a crafted link cannot steer the key elsewhere.
 */
export function bridgeOverride(): string | null {
  try {
    const v = new URLSearchParams(globalThis.location?.search ?? '').get('bridge');
    if (!v) return null;
    const u = new URL(v);
    if (u.protocol === 'http:' && (u.hostname === '127.0.0.1' || u.hostname === 'localhost')) {
      return u.origin;
    }
    return null;
  } catch {
    return null;
  }
}

/** Bridge base from an API base plus port. Null when no host exists. */
export function buildBridgeBase(
  apiBase: string,
  port: string,
  proto: string,
  host: string
): string | null {
  if (apiBase === 'demo') return null;
  if (apiBase === 'same') return host ? `${proto}//${host}:${port}` : null;
  try {
    const u = new URL(apiBase);
    return `${u.protocol}//${u.hostname}:${port}`;
  } catch {
    return null;
  }
}

/** Approve gate. Only a waiting approval enables the button. */
export function approveState(
  bridgeOk: boolean,
  approval: boolean | null
): { disabled: boolean; reason: string } {
  if (!bridgeOk) return { disabled: true, reason: '' };
  if (approval === true) return { disabled: false, reason: '' };
  if (approval === false) return { disabled: true, reason: nothingToApprove() };
  return { disabled: true, reason: approvalUnknown() };
}

/** Live bridge base. Override wins, else same tailnet host plus port. */
export function bridgeBase(): string | null {
  const over = bridgeOverride();
  if (over) return over;
  const loc = globalThis.location;
  return buildBridgeBase(API_BASE, loadBridgePort(), loc?.protocol ?? '', loc?.hostname ?? '');
}

/** Exact bridge error: server text wins, fallback stays plain. */
export async function bridgeError(res: Response): Promise<BridgeError> {
  let message = 'Bridge said no. Retry soon.';
  let retryAfter: number | null = null;
  try {
    const b = (await res.json()) as { message?: unknown; retry_after?: unknown };
    if (typeof b?.message === 'string' && b.message) message = b.message;
    if (typeof b?.retry_after === 'number') retryAfter = b.retry_after;
  } catch {
    /* keep the fallback */
  }
  return { message, status: res.status, retryAfter };
}

/** One authed bridge read. Throws BridgeError on any refusal. */
export async function bridgeGet<T>(path: string): Promise<T> {
  const base = bridgeBase();
  const key = loadBridgeKey();
  if (!base || !key) throw { message: 'Bridge key or link is missing. Open Settings and add both.', status: 0 };
  let res: Response;
  try {
    res = await fetch(`${base}${path}`, { headers: { Authorization: `Bearer ${key}` } });
  } catch {
    throw { message: 'Bridge is down. Check the link.', status: 0 } as BridgeError;
  }
  if (!res.ok) throw await bridgeError(res);
  return (await res.json()) as T;
}

/** One authed bridge write. Throws BridgeError on any refusal. */
export async function bridgePost<T>(path: string, body: unknown): Promise<T> {
  const base = bridgeBase();
  const key = loadBridgeKey();
  if (!base || !key) throw { message: 'Bridge key or link is missing. Open Settings and add both.', status: 0 };
  let res: Response;
  try {
    res = await fetch(`${base}${path}`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
  } catch {
    throw { message: 'Bridge is down. Check the link.', status: 0 } as BridgeError;
  }
  if (!res.ok) throw await bridgeError(res);
  return (await res.json()) as T;
}

/** Text rule for the Answer box: 1..1000 chars, one paragraph. */
export function answerError(text: string): string | null {
  if (!text.trim()) return 'Write a line first.';
  if (text.includes('\n\n')) return 'One paragraph only. No blank lines.';
  if ([...text.trim()].length > BRIDGE_TEXT_MAX) return `Keep ${BRIDGE_TEXT_MAX} chars or less.`;
  return null;
}

export type QuickAction = { id: string; label: string; text: string };

/**
 * One-tap sends. Each still needs preview plus a confirm code.
 * Digits and y/n need a visible approval prompt on the pane.
 */
export const QUICK_ACTIONS: QuickAction[] = [
  { id: 'q1', label: '1', text: '1' },
  { id: 'q2', label: '2', text: '2' },
  { id: 'q3', label: '3', text: '3' },
  { id: 'q4', label: '4', text: '4' },
  { id: 'yes', label: 'Yes', text: 'y' },
  { id: 'no', label: 'No', text: 'n' },
  { id: 'esc', label: 'Esc', text: BRIDGE_ESC },
  { id: 'continue', label: 'Continue', text: 'continue' }
];

/** Plain label for previewed text. Control bytes never show raw. */
export function previewLabel(text: string): string {
  if (text === BRIDGE_ESC) return '(Escape key)';
  if (!text) return '(Enter press, no text)';
  return text;
}

/** Confirm matches when the typed code equals the shown code. */
export function codeMatches(typed: string, code: string): boolean {
  return code.trim().length > 0 && typed.trim() === code.trim();
}

/** True when the preview window has passed. */
export function codeExpired(issuedAt: number, expiresIn: number, now = Date.now()): boolean {
  return now - issuedAt >= expiresIn * 1000;
}
