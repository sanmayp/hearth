/** Pull-to-refresh action plus the Updated stamp. Pages share both. */

/** Pull distance in px that fires a refresh. */
export const PULL_THRESHOLD = 64;

/**
 * Svelte pull-to-refresh action. It fires onRefresh after a downward
 * pull past the threshold while the page sits at the top. Multi-touch
 * never fires. Listeners are passive, so scrolling stays smooth.
 */
export function pullToRefresh(
  node: HTMLElement,
  onRefresh: () => void
): { destroy: () => void } {
  let startY: number | null = null;
  function start(e: TouchEvent): void {
    if (globalThis.window?.scrollY <= 0 && e.touches.length === 1) {
      startY = e.touches[0].clientY;
    } else {
      startY = null;
    }
  }
  function end(e: TouchEvent): void {
    if (startY === null) return;
    const dy = e.changedTouches[0].clientY - startY;
    startY = null;
    if (dy >= PULL_THRESHOLD) onRefresh();
  }
  node.addEventListener('touchstart', start, { passive: true });
  node.addEventListener('touchend', end, { passive: true });
  return {
    destroy(): void {
      node.removeEventListener('touchstart', start);
      node.removeEventListener('touchend', end);
    }
  };
}

/** "Updated 14:32." stamp from a load time. Local clock, zero padded. */
export function updatedStamp(at: Date | null): string {
  if (!at) return 'Never updated.';
  const p = (n: number): string => String(n).padStart(2, '0');
  return `Updated ${p(at.getHours())}:${p(at.getMinutes())}.`;
}
