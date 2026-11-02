<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr } from '$lib/api';
  import { demoFactory, demoMuse, type FactoryStatus, type MuseBoard } from '$lib/demo';

  type Group = 'prog' | 'next' | 'done';
  type WorkItem = {
    title: string;
    group: Group;
    detail: string;
    at: string | null;
    source: string;
  };

  const GROUP_LABEL: Record<Group, string> = {
    prog: 'In progress',
    next: 'Next',
    done: 'Completed'
  };
  const OPEN_GROUPS: Group[] = ['prog', 'next'];

  let data = $state<FactoryStatus>(demoFactory);
  let muse = $state<MuseBoard>(demoMuse);
  let live = $state(true);
  let loaded = $state(false);
  let showDone = $state(false);

  onMount(async () => {
    const res = await apiGetOr<FactoryStatus>('/factory/status', demoFactory);
    data = res.data;
    const m = await apiGetOr<MuseBoard>('/watch/muse', demoMuse);
    muse = m.data;
    live = res.live && m.live;
    loaded = true;
  });

  const lines = (body: string) => body.split('\n').map((l) => l.trim()).filter(Boolean);

  function groupOf(section: string): Group {
    if (section.startsWith('In flight') || section === 'Blocked') return 'prog';
    if (section.startsWith('Waits on owner')) return 'next';
    if (section.startsWith('Merged') || section === 'Fixed today') return 'done';
    return 'prog';
  }

  function workList(d: FactoryStatus): WorkItem[] {
    const out: WorkItem[] = [];
    for (const b of d.builds ?? []) {
      out.push({
        title: b.name,
        group: b.stage === 'done' ? 'done' : 'prog',
        detail: b.stage,
        at: null,
        source: 'builds.json'
      });
    }
    for (const s of d.state?.sections ?? []) {
      for (const line of lines(s.body)) {
        if (!line.startsWith('- ')) continue;
        out.push({
          title: line.slice(2),
          group: groupOf(s.title),
          detail: s.title,
          at: null,
          source: 'STATE.md'
        });
      }
    }
    for (const row of [...d.ledger].reverse()) {
      out.push({
        title: row.work,
        group: 'done',
        detail: row.lane,
        at: row.ts || null,
        source: 'ledger.jsonl'
      });
    }
    return out;
  }

  const inGroup = (items: WorkItem[], g: Group) => items.filter((i) => i.group === g);

  function when(ts: string): string {
    if (!ts) return '';
    const d = new Date(ts);
    return Number.isNaN(d.getTime()) ? ts : d.toLocaleString();
  }
</script>

<main>
  <h1>Factory</h1>
  {#if !live}<p class="demo">Sample data, not live.</p>{/if}
  {#if loaded}
    {@const items = workList(data)}
    <h2 class="listhead">Work</h2>
    {#each OPEN_GROUPS as g}
      {@const list = inGroup(items, g)}
      <h3 class="subhead">{GROUP_LABEL[g]} <span class="num">{list.length}</span></h3>
      {#each list as item, i (`${g}-${i}`)}
        <section class="card">
          <p>{item.title}</p>
          <p class="age">{item.detail}{item.at ? ` · ${when(item.at)}` : ''} · from {item.source}</p>
        </section>
      {/each}
    {/each}
    <label class="check">
      <input type="checkbox" bind:checked={showDone} />
      Show completed ({inGroup(items, 'done').length})
    </label>
    {#if showDone}
      <h3 class="subhead">{GROUP_LABEL.done} <span class="num">{inGroup(items, 'done').length}</span></h3>
      {#each inGroup(items, 'done') as item, i (`done-${i}`)}
        <section class="card">
          <p>{item.title}</p>
          <p class="age">{item.detail}{item.at ? ` · ${when(item.at)}` : ''} · from {item.source}</p>
        </section>
      {/each}
    {/if}
    {#if data.state}
      <h2 class="listhead">State</h2>
      {#if data.state.sections.length > 0}
        {#each data.state.sections as s (s.title)}
          <section class="card">
            <h3>{s.title}</h3>
            {#each lines(s.body) as line, i (`${s.title}-${i}`)}
              <p>{line}</p>
            {/each}
          </section>
        {/each}
      {:else if data.state.raw}
        <section class="card"><pre>{data.state.raw}</pre></section>
      {/if}
    {/if}
    {#if data.builds}
      <h2 class="listhead">Builds <span class="num">{data.builds.length}</span></h2>
      {#each data.builds as b (b.id)}
        <section class="card">
          <h3>{b.name}</h3>
          <p>{b.stage} · {b.note}</p>
        </section>
      {/each}
    {/if}
    <h2 class="listhead">Runs <span class="num">{data.ledger.length}</span></h2>
    {#each data.ledger.slice(-10).reverse() as row, i (i)}
      <section class="card">
        <h3>{row.lane} <span class="age num">{when(row.ts)}</span></h3>
        <p>{row.work}</p>
        <p class="age">{row.outcome}</p>
      </section>
    {/each}
    <h2 class="listhead">Agents</h2>
    <p class="age">Read-only. Status only, no replies here.</p>
    {#each muse.sessions as s (s.pane)}
      <section class="card">
        <h3>{s.name} <span class="age num">{s.state}{s.changed_at ? ` · since ${s.changed_at}` : ''}</span></h3>
        <p>{s.question ?? 'No open question.'}</p>
        {#if s.recommendation}<p class="age">Suggests: {s.recommendation}</p>{/if}
      </section>
    {/each}
  {:else}
    <section class="card skeleton" aria-busy="true">
      <div class="bar"></div>
      <div class="bar short"></div>
    </section>
  {/if}
</main>

<style>
  main {
    max-width: 40rem;
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
  .listhead {
    font-size: var(--text-sm);
    color: var(--paper-dim);
    margin: 1.5rem 0 0;
  }
  .subhead {
    font-size: var(--text-sm);
    margin: 0.75rem 0 0;
  }
  .check {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-top: 0.75rem;
    font-size: var(--text-sm);
  }
  .card {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1rem 1.25rem;
    margin-top: 0.6rem;
  }
  .card h3 {
    font-size: var(--text-md);
    margin: 0;
  }
  .card p {
    margin: 0.25rem 0 0;
  }
  .card pre {
    white-space: pre-wrap;
    font: inherit;
    margin: 0;
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
