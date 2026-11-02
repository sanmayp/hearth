<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr } from './api';
  import { EMPTY_USAGE, barWidth, totalsLine, type UsageFeed } from './aiusage';

  let feed = $state<UsageFeed>(EMPTY_USAGE);
  let live = $state(false);
  let stale = $state(false);
  let at = $state<Date | null>(null);

  const names = $derived(Object.keys(feed.providers).sort());
  const maxCalls = $derived(
    names.reduce((m, n) => Math.max(m, feed.providers[n]?.calls ?? 0), 0)
  );

  onMount(async () => {
    const res = await apiGetOr<UsageFeed>('/ai/usage', EMPTY_USAGE);
    feed = res.data ?? EMPTY_USAGE;
    live = res.live;
    stale = res.stale;
    at = res.at ?? null;
  });
</script>

<section class="card" aria-label="AI usage">
  <h2>AI usage</h2>
  {#if feed.totals.calls === 0}
    <p>No runs yet. Run the batch to fill this card.</p>
  {:else}
    <ul>
      {#each names as name (name)}
        {@const t = feed.providers[name]}
        <li>
          <div class="head">
            <b>{name}</b>
            <span>{t.calls} calls · {t.live} live · {t.mock} mock</span>
          </div>
          <div
            class="bar"
            role="img"
            aria-label="{name}: {t.calls} calls"
          >
            <span style="width:{barWidth(t.calls, maxCalls)}%"></span>
          </div>
          <p class="age">
            {t.tokens_in + t.tokens_out} tokens · p50 {t.p50_ms} ms · p95 {t.p95_ms} ms{#if t.errors > 0}
              · {t.errors} errors{/if}
          </p>
        </li>
      {/each}
    </ul>
    <p>{totalsLine(feed.totals)}</p>
  {/if}
  {#if stale && at}<p class="age">Last data, stale since {at.toLocaleString()}.</p>{/if}
  {#if !live && feed.totals.calls > 0 && !stale}<p class="age">Link down. Kept data shown.</p>{/if}
</section>

<style>
  .card {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1.25rem 1.5rem;
    margin-top: 1rem;
  }
  .card h2 {
    font-size: var(--text-md);
    margin: 0 0 0.5rem;
  }
  .card p {
    margin: 0.25rem 0;
  }
  .age {
    color: var(--paper-dim);
    font-size: var(--text-sm);
  }
  ul {
    list-style: none;
    margin: 0.5rem 0;
    padding: 0;
  }
  li + li {
    margin-top: 0.75rem;
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-size: var(--text-sm);
  }
  .head span {
    color: var(--paper-dim);
  }
  .bar {
    background: var(--ink);
    border-radius: 0.4rem;
    height: 0.55rem;
    margin-top: 0.3rem;
    overflow: hidden;
  }
  .bar span {
    display: block;
    background: var(--accent);
    height: 100%;
  }
</style>
