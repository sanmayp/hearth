<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr } from '$lib/api';
  import {
    demoBuilds,
    demoAgents,
    demoMuse,
    type DemoBuild,
    type DemoAgent,
    type BuildStage,
    type MuseBoard
  } from '$lib/demo';

  const stages: BuildStage[] = ['made', 'tested', 'live', 'checked', 'done'];

  let builds = $state<DemoBuild[]>([]);
  let agents = $state<DemoAgent[]>([]);
  let muse = $state<MuseBoard>(demoMuse);
  let live = $state(true);
  let loaded = $state(false);

  type Feed = { builds: DemoBuild[]; agents: DemoAgent[] };

  // First the live file. Then the API. Demo fills gaps.
  async function loadFeed(): Promise<Feed | null> {
    try {
      const res = await fetch('/builds-status.json');
      if (!res.ok) return null;
      const feed = (await res.json()) as Partial<Feed>;
      if (!Array.isArray(feed.builds) || !Array.isArray(feed.agents)) return null;
      return { builds: feed.builds, agents: feed.agents };
    } catch {
      return null;
    }
  }

  onMount(async () => {
    // Sessions always read live. The feed below never skips this.
    const m = await apiGetOr<MuseBoard>('/watch/muse', demoMuse);
    muse = m.data;
    const feed = await loadFeed();
    if (feed) {
      builds = feed.builds;
      agents = feed.agents;
      live = m.live;
      loaded = true;
      return;
    }
    const b = await apiGetOr<DemoBuild[]>('/builds', demoBuilds);
    builds = b.data;
    const a = await apiGetOr<DemoAgent[]>('/builds/agents', demoAgents);
    agents = a.data;
    live = b.live && a.live && m.live;
    loaded = true;
  });

  const atStage = (s: BuildStage) => builds.filter((b) => b.stage === s);
  const dot = (s: string) =>
    s === 'working'
      ? 'var(--ok)'
      : s === 'blocked' || s === 'gone'
        ? 'var(--bad)'
        : s === 'done'
          ? 'var(--paper-dim)'
          : 'var(--warn)';
</script>

<main>
  <h1>Builds</h1>
  <img class="mast" src="/art/module-builds.webp" alt="" width="1024" height="688" />
  {#if !live}<p class="demo">Sample data. The live board lands with the factory feed.</p>{/if}
  {#if loaded}
    <div class="board">
      {#each stages as s}
        <section class="lane" aria-label="Stage {s}">
          <h2>{s} <span class="num">{atStage(s).length}</span></h2>
          {#each atStage(s) as b (b.id)}
            <div class="chip">
              <p class="name">{b.name}</p>
              <p class="age">{b.note}</p>
            </div>
          {/each}
        </section>
      {/each}
    </div>
    <section class="card agents">
      <img class="state" src="/art/module-agents.webp" alt="" width="1024" height="688" />
      <h2>Coding agents</h2>
      {#each agents as a (a.id)}
        <p><span class="pip" style="background: {dot(a.state)}"></span>{a.name} · {a.state} · {a.job}</p>
      {/each}
    </section>
    <section class="card muse" aria-label="Muse sessions">
      <h2>Muse sessions</h2>
      <p class="age">Read-only. {muse.log_at ? `Newest log line: ${muse.log_at}.` : 'No log yet.'}</p>
      {#each muse.sessions as s (s.pane)}
        <div class="row">
          <p>
            <span class="pip" style="background: {dot(s.state)}"></span>{s.name} · {s.state}{s.changed_at
              ? ` · since ${s.changed_at}`
              : ''}
          </p>
          {#each s.alerts as a}
            <p class="alert">{a}</p>
          {/each}
          <p class="age">{s.stage ?? 'No stage report yet.'}</p>
        </div>
      {/each}
    </section>
  {:else}
    <section class="card skeleton" aria-busy="true">
      <div class="bar"></div>
      <div class="bar short"></div>
    </section>
  {/if}
</main>

<style>
  main {
    max-width: 64rem;
    margin: 0 auto;
    padding: 2rem 1.25rem 4rem;
  }
  h1 {
    font-size: var(--text-display);
    margin: 0;
  }
  .age {
    color: var(--paper-dim);
    font-size: var(--text-sm);
  }
  .demo {
    color: var(--accent);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .board {
    display: grid;
    gap: 0.75rem;
    margin-top: 1rem;
  }
  @media (min-width: 56rem) {
    .board {
      grid-template-columns: repeat(5, minmax(0, 1fr));
    }
  }
  .lane {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 0.9rem 1rem;
  }
  .lane h2 {
    font-size: var(--text-sm);
    text-transform: capitalize;
    margin: 0 0 0.5rem;
    color: var(--paper-dim);
  }
  .chip {
    background: var(--ink);
    border-radius: 0.75rem;
    padding: 0.6rem 0.8rem;
    margin-top: 0.5rem;
  }
  .chip .name {
    margin: 0;
    font-size: var(--text-sm);
  }
  .chip .age {
    margin: 0.15rem 0 0;
  }
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
  .state {
    display: block;
    width: 100%;
    height: 8rem;
    object-fit: cover;
    object-position: center;
    border-radius: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .pip {
    display: inline-block;
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    margin-right: 0.5rem;
  }
  .muse .row {
    border-top: 1px solid var(--ink);
    padding-top: 0.5rem;
    margin-top: 0.5rem;
  }
  .muse .row:first-of-type {
    border-top: none;
    padding-top: 0;
  }
  .alert {
    color: var(--warn);
  }
  .skeleton .bar {
    height: 1rem;
    border-radius: 0.5rem;
    background: var(--paper-dim);
    opacity: 0.35;
    margin: 0.5rem 0;
  }
  .skeleton .short {
    width: 40%;
  }
</style>
