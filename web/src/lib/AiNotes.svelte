<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr } from './api';
  import {
    EMPTY_RESULTS,
    laneLabel,
    type ResultsFeed
  } from './aiusage';

  type Group = { task: string; title: string; hint: string };

  const groups: Group[] = [
    { task: 'explain', title: 'Lane notes', hint: 'One plain explanation per lane.' },
    { task: 'digest', title: 'Nightly digest', hint: 'Agent log lines, digested.' },
    { task: 'suppliers', title: 'Filing evidence', hint: 'Supplier and customer sentences.' }
  ];

  let rows = $state<Record<string, ResultsFeed['rows']>>({});
  let loaded = $state(false);

  onMount(async () => {
    const out: Record<string, ResultsFeed['rows']> = {};
    for (const g of groups) {
      const res = await apiGetOr<ResultsFeed>(`/ai/results?task=${g.task}&limit=5`, EMPTY_RESULTS);
      out[g.task] = res.data?.rows ?? [];
    }
    rows = out;
    loaded = true;
  });
</script>

<section class="card" aria-label="AI notes">
  <h2>AI notes</h2>
  {#if !loaded}
    <p class="age">Loading notes.</p>
  {:else}
    {#each groups as g (g.task)}
      <h3>{g.title}</h3>
      <p class="age">{g.hint}</p>
      {#if (rows[g.task] ?? []).length === 0}
        <p>No runs yet. Run the batch to fill this list.</p>
      {:else}
        <ul>
          {#each rows[g.task] ?? [] as row (row.id)}
            <li>
              <div class="head">
                <b>{laneLabel(row.file)}</b>
                <span>{row.provider} · {row.model} · {row.mock ? 'mock' : 'live'}</span>
              </div>
              <p>{row.output || 'No reply recorded.'}</p>
            </li>
          {/each}
        </ul>
      {/if}
    {/each}
  {/if}
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
  .card h3 {
    font-size: var(--text-sm);
    margin: 1rem 0 0;
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
    margin: 0.5rem 0 0;
    padding: 0;
  }
  li + li {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--ink);
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-size: var(--text-sm);
  }
  .head span {
    color: var(--paper-dim);
    text-align: right;
  }
</style>
