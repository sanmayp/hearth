<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, apiPost, type ApiError } from '$lib/api';
  import { demoToday } from '$lib/demo';
  import {
    demoFeatures,
    splitWhereLinks,
    stepProgressLabel,
    todayIso,
    yourMoveList,
    type FeatureRow,
    type FeaturesLedger,
    type YourMoveItem
  } from '$lib/features';
  import {
    barShare,
    buildRows,
    dateLine,
    dayCounts,
    doneToday,
    greeting,
    heroSnoozed,
    loadDoneLog,
    recordDone,
    ringFrac,
    ringOffset,
    snoozeHero,
    stepCounts,
    type HomeRow
  } from '$lib/home';
  import type { TodayView } from '$lib/today';
  import type { NeedsYouFeed } from '$lib/needs-you';
  import { pickNext } from '$lib/nextstep';
  import { loadUse } from '$lib/screenuse';
  import StepRow from '$lib/StepRow.svelte';

  const EMPTY_FEED: NeedsYouFeed = { threads: [], unused: [], events: [], log_at: null };

  let today = $state<TodayView>(demoToday);
  let features = $state<FeatureRow[]>(demoFeatures.features);
  let feed = $state<NeedsYouFeed>(EMPTY_FEED);
  let live = $state(false);
  let loaded = $state(false);
  let hideDone = $state(false);
  let snoozed = $state(true);
  let logTick = $state(0);
  let busyKey = $state<string | null>(null);
  let rowErr = $state<Record<string, string>>({});
  let loadErr = $state('');

  const todayStr = todayIso();
  const moves = $derived(yourMoveList(features));
  const sc = $derived(stepCounts(moves, features, todayStr));
  const logN = $derived.by(() => {
    void logTick;
    return doneToday(loadDoneLog(), todayStr);
  });
  const counts = $derived(dayCounts(today, sc.open, sc.doneToday, logN));
  const frac = $derived(ringFrac(counts.done, counts.total));
  const rows = $derived(buildRows(today, features, moves, todayStr, hideDone));
  const hero = $derived(snoozed ? null : pickNext(feed, features, loadUse()));
  const calm = $derived(counts.total > 0 && counts.done >= counts.total);
  const barMax = $derived(Math.max(counts.stalled, counts.waiting, counts.done, 1));

  function heroHref(): string {
    if (!hero) return '/';
    return hero.ctaHref.startsWith('#') ? `/needs-you${hero.ctaHref}` : hero.ctaHref;
  }

  async function loadAll(): Promise<void> {
    loadErr = '';
    try {
      const [t, f, n] = await Promise.all([
        apiGetOr<TodayView>('/today', demoToday),
        apiGetOr<FeaturesLedger>('/watch/features', demoFeatures),
        apiGetOr<NeedsYouFeed>('/watch/needs-you', EMPTY_FEED)
      ]);
      today = t.data;
      features = f.data?.features ?? [];
      feed = n.data ?? EMPTY_FEED;
      live = t.live || f.live || n.live;
    } catch (e) {
      loadErr = `Failed: ${(e as ApiError)?.message ?? 'Home did not load. Retry.'}`;
    } finally {
      snoozed = heroSnoozed(todayStr);
      loaded = true;
    }
  }

  function later(): void {
    snoozeHero(todayStr);
    snoozed = true;
  }

  async function toggleListRow(row: HomeRow): Promise<void> {
    const path = row.kind === 'reminder' ? '/reminders' : '/tasks';
    const id = row.key.split(':')[1] ?? '';
    busyKey = row.key;
    rowErr = { ...rowErr, [row.key]: '' };
    try {
      await apiPost(`${path}/${encodeURIComponent(id)}/done`, {});
      recordDone(row.key, todayStr);
      logTick += 1;
      const t = await apiGetOr<TodayView>('/today', demoToday);
      today = t.data;
    } catch (e) {
      rowErr = {
        ...rowErr,
        [row.key]: `Failed: ${(e as ApiError)?.message ?? 'Nothing changed. Retry.'}`
      };
    } finally {
      busyKey = null;
    }
  }

  async function toggleStep(
    id: string,
    index: number,
    want: boolean
  ): Promise<{ done: boolean; error: string | null }> {
    busyKey = `step:${id}:${index}`;
    try {
      const res = await apiPost<{ done?: boolean; done_at?: string }>(
        `/watch/features/${encodeURIComponent(id)}/step`,
        { index, done: want }
      );
      const nextDone = res.done ?? want;
      features = features.map((f) => {
        if (f.id !== id || !Array.isArray(f.steps)) return f;
        return {
          ...f,
          steps: f.steps.map((s, i) =>
            i !== index ? s : { ...s, done: nextDone, done_at: nextDone ? todayStr : '' }
          )
        };
      });
      return { done: nextDone, error: null };
    } catch (err) {
      const apiErr = err as ApiError;
      const offline = apiErr?.status === undefined || apiErr.status === 0;
      return { done: want, error: offline ? null : (apiErr?.message ?? 'Retry.') };
    } finally {
      busyKey = null;
    }
  }

  function stepDoneAt(m: YourMoveItem): string {
    const s = features.find((f) => f.id === m.featureId)?.steps?.[m.stepIndex];
    return s?.done_at ?? '';
  }

  onMount(() => void loadAll());
</script>

<main>
  <div class="greet">
    <h1>{greeting(new Date().getHours())},</h1>
    <p>{dateLine()}</p>
  </div>
  {#if !live && loaded}<p class="demo">Sample data.</p>{/if}
  {#if loadErr}<p class="err">{loadErr}</p>{/if}

  {#if loaded}
    {#if calm}
      <div class="calmbox">
        <svg class="bigring" viewBox="0 0 200 200" role="img" aria-label="{counts.done} of {counts.total} done">
          <circle cx="100" cy="100" r="88" fill="none" stroke="color-mix(in srgb, var(--paper) 12%, transparent)" stroke-width="16" />
          <circle cx="100" cy="100" r="88" fill="none" stroke="var(--ok)" stroke-width="16" stroke-linecap="round" stroke-dasharray="552.9" stroke-dashoffset="0" transform="rotate(-90 100 100)" />
          <text x="100" y="92" text-anchor="middle" fill="var(--paper)" font-size="26">Today</text>
          <text x="100" y="128" text-anchor="middle" fill="var(--paper)" font-size="30" font-weight="700">
            {counts.done} of {counts.total}
          </text>
        </svg>
        <h2>All calm.</h2>
        <p>Nothing needs you.</p>
        <a class="btn primary" href="#homerows">See today</a>
      </div>
    {:else}
      <svg class="bigring" viewBox="0 0 200 200" role="img" aria-label="Today, {counts.done} of {counts.total} done">
        <circle cx="100" cy="100" r="88" fill="none" stroke="color-mix(in srgb, var(--paper) 12%, transparent)" stroke-width="16" />
        <circle
          class:glow={frac > 0}
          cx="100" cy="100" r="88" fill="none" stroke="var(--accent)" stroke-width="16"
          stroke-linecap="round" stroke-dasharray="552.9" stroke-dashoffset={ringOffset(frac)}
          transform="rotate(-90 100 100)"
        />
        <text x="100" y="92" text-anchor="middle" fill="var(--paper)" font-size="26">Today</text>
        <text x="100" y="128" text-anchor="middle" fill="var(--paper)" font-size="30" font-weight="700">
          <tspan fill="var(--accent)">{counts.done}</tspan> of {counts.total}
        </text>
      </svg>
    {/if}

    <div class="minibars" aria-label="Counts">
      <figure>
        <div class="mb"><i style="width:{barShare(counts.stalled, barMax)}%;background:var(--bad);"></i></div>
        <figcaption><strong>{counts.stalled}</strong> stalled</figcaption>
      </figure>
      <figure>
        <div class="mb"><i style="width:{barShare(counts.waiting, barMax)}%;background:var(--wait);"></i></div>
        <figcaption><strong>{counts.waiting}</strong> waiting</figcaption>
      </figure>
      <figure>
        <div class="mb"><i style="width:{barShare(counts.done, barMax)}%;background:var(--ok);"></i></div>
        <figcaption><strong>{counts.done}</strong> done</figcaption>
      </figure>
    </div>

    {#if hero && !calm}
      <section class="hero" aria-label="Do this next">
        <h2>Do this next</h2>
        <p class="wh">
          <span><b>{hero.name}</b></span>
          {#if hero.where}<span>
              Where: {#each splitWhereLinks(hero.where) as part}{#if part.href}<a
                  href={part.href}>{part.text}</a
                >{:else}{part.text}{/if}{/each}
            </span>{/if}
          {#if hero.how}<span>How: {hero.how}</span>{/if}
        </p>
        <a class="btn-big" href={heroHref()}>{hero.ctaLabel}</a>
        <button class="later" type="button" onclick={later}>Later</button>
      </section>
    {/if}

    {#if counts.total === 0}
      <p class="empty">Nothing tracked yet. <a href="/tasks">Add a task</a> to start.</p>
    {:else}
      <ul class="rows" id="homerows">
        {#each rows as row (row.key)}
          <li>
            {#if row.kind === 'step' && row.move}
              <StepRow
                item={{ ...row.move, stepSummary: stepProgressLabel(features.find((f) => f.id === row.featureId) ?? features[0]) }}
                done={row.done}
                doneAt={row.move ? stepDoneAt(row.move) : ''}
                busy={busyKey === row.key}
                onToggle={toggleStep}
              />
            {:else if row.kind === 'health'}
              <a class="hrow" href="/health">
                <span class="t">{row.title}</span>
                <span class="tag">health</span>
              </a>
            {:else}
              <div class="hrow">
                <button
                  type="button"
                  class="check"
                  disabled={busyKey === row.key}
                  aria-label={row.done ? 'Mark not done' : 'Mark done'}
                  onclick={() => void toggleListRow(row)}
                >
                  {row.done ? '✓' : ''}
                </button>
                <span class="t">{row.title}{#if row.sub}<small>{row.sub}</small>{/if}</span>
              </div>
              {#if rowErr[row.key]}<p class="err">{rowErr[row.key]}</p>{/if}
            {/if}
          </li>
        {/each}
      </ul>
      <button class="pillbtn" type="button" onclick={() => (hideDone = !hideDone)}>
        {hideDone ? 'Show completed' : 'Hide completed'}
      </button>
    {/if}
  {:else}
    <p class="age">Today loads.</p>
  {/if}
</main>

<style>
  main {
    max-width: 40rem;
    margin: 0 auto;
    padding: 2rem 1.25rem 6rem;
  }
  .greet h1 {
    font-family: var(--font-display);
    font-size: var(--text-display);
    margin: 0;
    line-height: 1.05;
  }
  .greet p {
    margin: 0.25rem 0 0;
    font-size: var(--text-md);
    color: var(--paper-dim);
  }
  .demo {
    color: var(--warn);
    font-size: var(--text-sm);
  }
  .err {
    color: var(--bad);
    font-size: var(--text-sm);
  }
  .age {
    color: var(--paper-dim);
    font-size: var(--text-sm);
  }
  .bigring {
    display: block;
    width: min(20rem, 80vw);
    margin: 1.5rem auto 0;
  }
  .bigring .glow {
    filter: drop-shadow(0 0 12px var(--accent));
  }
  .calmbox {
    text-align: center;
  }
  .calmbox h2 {
    font-size: var(--text-lg);
    margin: 0.5rem 0 0;
  }
  .calmbox p {
    color: var(--paper-dim);
    margin: 0.25rem 0 0.75rem;
  }
  .btn {
    display: inline-block;
    background: var(--accent);
    color: var(--accent-ink);
    border-radius: 999px;
    padding: 0.75rem 1.75rem;
    font-weight: 700;
    text-decoration: none;
    min-height: 3rem;
  }
  .minibars {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
    margin-top: 1.5rem;
  }
  .minibars figure {
    margin: 0;
  }
  .mb {
    height: 0.5rem;
    border-radius: 999px;
    background: color-mix(in srgb, var(--paper) 12%, transparent);
    overflow: hidden;
  }
  .mb i {
    display: block;
    height: 100%;
    border-radius: 999px;
  }
  .minibars figcaption {
    margin-top: 0.35rem;
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .hero {
    margin-top: 1.5rem;
    border: 2px solid var(--accent);
    border-radius: 1.5rem;
    padding: 1.5rem 1.25rem;
    text-align: center;
    background: var(--ink-raise);
  }
  .hero h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  .hero .wh {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: var(--text-sm);
    margin: 0.75rem 0 0;
  }
  .btn-big {
    display: block;
    margin-top: 1rem;
    background: var(--accent);
    color: var(--accent-ink);
    border-radius: 999px;
    padding: 1rem;
    font-size: var(--text-md);
    font-weight: 700;
    text-decoration: none;
    min-height: 3.5rem;
  }
  .later {
    margin-top: 0.5rem;
    background: none;
    border: 0;
    color: var(--paper-dim);
    font: inherit;
    font-size: var(--text-sm);
    text-decoration: underline;
    min-height: 3rem;
    padding: 0 1rem;
  }
  .rows {
    list-style: none;
    margin: 1.5rem 0 0;
    padding: 0;
    scroll-margin-top: 1rem;
  }
  .rows li {
    padding: 1.125rem 0;
    border-bottom: 2px dashed color-mix(in srgb, var(--paper-dim) 45%, transparent);
  }
  .rows li:last-child {
    border-bottom: 0;
  }
  .hrow {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    color: inherit;
    text-decoration: none;
  }
  a.hrow {
    min-height: 3rem;
  }
  .check {
    flex: none;
    width: 3rem;
    height: 3rem;
    border-radius: 50%;
    border: 2px solid var(--accent);
    background: transparent;
    color: var(--accent);
    font-size: var(--text-md);
    font-weight: 700;
  }
  .check:disabled {
    opacity: 0.5;
  }
  .hrow .t {
    font-size: var(--text-md);
    display: flex;
    flex-direction: column;
  }
  .hrow .t small {
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .hrow .tag {
    margin-left: auto;
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .pillbtn {
    display: block;
    margin: 1rem auto 0;
    background: transparent;
    border: 1px solid var(--paper-dim);
    color: var(--paper);
    border-radius: 999px;
    padding: 0.75rem 1.5rem;
    font: inherit;
    font-size: var(--text-sm);
    min-height: 3rem;
  }
  .empty {
    margin-top: 1.5rem;
    font-size: var(--text-md);
  }
</style>
