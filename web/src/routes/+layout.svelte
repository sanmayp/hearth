<script lang="ts">
  import '../lib/tokens.css';
  import '@fontsource-variable/fraunces';
  import '@fontsource-variable/public-sans';
  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';

  let { children } = $props();

  let updateReady = $state(false);
  let applyUpdate: (() => void) | null = $state(null);
  let moreOpen = $state(false);
  let online = $state(true);

  const areas = [
    { href: '/', label: 'Today' },
    { href: '/dev', label: 'Dev' },
    { href: '/features', label: 'Project' },
    { href: '/threads', label: 'Talk' },
    { href: '/today', label: 'Today (classic)' },
    { href: '/reminders', label: 'Reminders' },
    { href: '/tasks', label: 'Tasks' },
    { href: '/health', label: 'Health' },
    { href: '/inventory', label: 'Items' },
    { href: '/business', label: 'Business' },
    { href: '/factory', label: 'Factory' },
    { href: '/builds', label: 'Builds' },
    { href: '/needs-you', label: 'Needs you' },
    { href: '/settings', label: 'Settings' }
  ];
  const barFour = areas.slice(0, 4);
  const sheetRest = areas.slice(4);

  async function trackScreen(path: string): Promise<void> {
    try {
      const { recordOpen, loadSync, dueForSync, markSynced, syncOpen } =
        await import('$lib/screenuse');
      const { loadToken } = await import('$lib/api');
      const rec = recordOpen(path)[path];
      if (!rec || !loadToken()) return;
      if (!dueForSync(loadSync(), path)) return;
      if (await syncOpen(path, rec.first, rec.last)) markSynced(path);
    } catch {
      // Tracking never breaks the page.
    }
  }

  afterNavigate((nav) => {
    const to = nav.to?.url.pathname;
    if (to) trackScreen(to);
  });

  onMount(async () => {
    // Theme pin for shots and tests (?theme=light|dark).
    const theme = new URLSearchParams(location.search).get('theme');
    if (theme === 'light' || theme === 'dark') document.documentElement.dataset.theme = theme;
    // App update check. Once on open, once a day while open. No service.
    try {
      const { runUpdateCheck } = await import('$lib/update');
      runUpdateCheck().catch(() => {});
      window.setInterval(() => runUpdateCheck().catch(() => {}), 24 * 60 * 60 * 1000);
    } catch {
      /* PWA mode: nothing to check */
    }
    // Offline banner. Shows art, keeps last data.
    online = navigator.onLine;
    const mark = () => (online = navigator.onLine);
    window.addEventListener('online', mark);
    window.addEventListener('offline', mark);
    // Screen-open tracking. Records on device, syncs hourly with a key.
    trackScreen(location.pathname);
    // Service worker prompt. Never reloads mid-task.
    const { registerSW } = await import('virtual:pwa-register');
    applyUpdate =
      registerSW({
        onNeedRefresh() {
          updateReady = true;
        }
      }) ?? null;
  });
</script>

<div class="shell">{@render children()}</div>

{#if !online}
  <div class="offline" role="status">
    <img src="/art/offline-state.webp" alt="" width="768" height="768" />
    <span>No link. Hearth keeps your last data.</span>
  </div>
{/if}

<nav aria-label="Areas">
  <span class="brand"><span class="dot" aria-hidden="true"></span>Hearth</span>
  {#each barFour as a}
    <a href={a.href}>{a.label}</a>
  {/each}
  {#each sheetRest as a}
    <a class="extra" href={a.href}>{a.label}</a>
  {/each}
  <button class="more" onclick={() => (moreOpen = true)}>More</button>
</nav>

{#if moreOpen}
  <div class="sheet" role="dialog" aria-label="More areas">
    {#each areas as a}
      <a href={a.href} onclick={() => (moreOpen = false)}>{a.label}</a>
    {/each}
    <button onclick={() => (moreOpen = false)}>Close</button>
  </div>
{/if}

{#if updateReady}
  <div class="update-toast" role="status">
    <span>A new Hearth waits.</span>
    <button
      onclick={() => {
        updateReady = false;
        applyUpdate?.();
      }}>Use it now</button
    >
  </div>
{/if}

<style>
  /* Phone: bottom bar with 4 areas plus More. */
  nav {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    justify-content: flex-start;
    gap: 1.25rem;
    overflow-x: auto;
    scrollbar-width: none;
    background: var(--ink-raise);
    border-top: 1px solid var(--paper-dim);
    padding: 0.6rem 1.25rem calc(0.6rem + env(safe-area-inset-bottom));
  }
  nav::-webkit-scrollbar {
    display: none;
  }
  nav a {
    flex: none;
    color: var(--paper-dim);
    text-decoration: none;
    font-size: var(--text-sm);
  }
  .brand {
    display: none;
  }
  .extra {
    display: none;
  }
  .more {
    flex: none;
    background: none;
    border: 0;
    color: var(--paper-dim);
    font: inherit;
    font-size: var(--text-sm);
    padding: 0;
  }
  .sheet {
    position: fixed;
    left: 1rem;
    right: 1rem;
    bottom: calc(3.5rem + env(safe-area-inset-bottom));
    display: grid;
    gap: 0.25rem;
    background: var(--ink-raise);
    border: 1px solid var(--paper-dim);
    border-radius: 1rem;
    padding: 1rem 1.25rem;
    z-index: 20;
  }
  .sheet a {
    color: var(--paper);
    text-decoration: none;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--ink);
  }
  .sheet button {
    margin-top: 0.5rem;
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    border-radius: 0.5rem;
    padding: 0.5rem 1rem;
    font: inherit;
  }
  /* Desktop: left rail with all areas. */
  @media (min-width: 48rem) {
    .shell {
      padding-left: 13rem;
    }
    nav {
      top: 0;
      bottom: 0;
      right: auto;
      width: 13rem;
      flex-direction: column;
      justify-content: flex-start;
      gap: 0.25rem;
      overflow: visible;
      border-top: 0;
      border-right: 1px solid var(--paper-dim);
      padding: 1.5rem 1.25rem;
    }
    nav a {
      font-size: var(--text-md);
      padding: 0.5rem 0.75rem;
      border-radius: 0.5rem;
    }
    .brand {
      display: flex;
      align-items: center;
      gap: 0.6rem;
      font-family: var(--font-display);
      font-size: var(--text-lg);
      margin-bottom: 1rem;
      padding: 0 0.75rem;
    }
    .dot {
      width: 0.9rem;
      height: 0.9rem;
      border-radius: 50%;
      background: radial-gradient(circle at 35% 35%, #ffd9a0, var(--accent) 60%, #7a4a12);
    }
    .extra {
      display: block;
    }
    .more {
      display: none;
    }
    .sheet {
      display: none;
    }
  }
  .offline {
    position: fixed;
    left: 50%;
    bottom: calc(4.5rem + env(safe-area-inset-bottom));
    transform: translateX(-50%);
    display: flex;
    gap: 0.75rem;
    align-items: center;
    background: var(--ink-raise);
    border: 1px solid var(--warn);
    border-radius: 0.75rem;
    padding: 0.6rem 1rem 0.6rem 0.6rem;
    z-index: 30;
    font-size: var(--text-sm);
  }
  .offline img {
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.5rem;
    object-fit: cover;
  }
  .update-toast {
    position: fixed;
    left: 50%;
    bottom: calc(4.5rem + env(safe-area-inset-bottom));
    transform: translateX(-50%);
    display: flex;
    gap: 1rem;
    align-items: center;
    background: var(--ink-raise);
    border: 1px solid var(--accent);
    border-radius: 0.75rem;
    padding: 0.75rem 1rem;
    z-index: 30;
  }
  .update-toast button {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    border-radius: 0.5rem;
    padding: 0.5rem 1rem;
    font: inherit;
  }
  @media (min-width: 48rem) {
    .update-toast {
      bottom: 1rem;
      left: calc(50% + 6.5rem);
    }
  }
</style>
