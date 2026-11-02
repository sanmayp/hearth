/** One clipboard copy. True when the text lands. Never throws. */
export async function copyText(text: string): Promise<boolean> {
  try {
    const nav = globalThis.navigator as
      | (Navigator & { clipboard?: { writeText: (t: string) => Promise<void> } })
      | undefined;
    if (nav?.clipboard?.writeText) {
      await nav.clipboard.writeText(text);
      return true;
    }
  } catch {
    /* fall through to the legacy path */
  }
  try {
    const doc = globalThis.document;
    const area = doc.createElement('textarea');
    area.value = text;
    doc.body.appendChild(area);
    area.select();
    const ok = doc.execCommand('copy');
    area.remove();
    return ok;
  } catch {
    return false;
  }
}
