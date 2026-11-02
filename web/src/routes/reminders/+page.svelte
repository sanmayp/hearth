<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, apiPost } from '$lib/api';
  import { demoReminders, type DemoReminder } from '$lib/demo';

  let items = $state<DemoReminder[]>([]);
  let live = $state(true);
  let stale = $state(false);
  let loaded = $state(false);
  let title = $state('');
  let when = $state('');
  let repeat = $state('');

  async function load() {
    const res = await apiGetOr<DemoReminder[]>('/reminders', demoReminders);
    items = res.data;
    live = res.live;
    stale = res.stale;
    loaded = true;
  }

  onMount(load);

  async function add() {
    if (!title.trim()) return;
    await apiPost('/reminders', {
      title: title.trim(),
      at: when ? new Date(when).toISOString() : null,
      rrule: repeat || null
    });
    title = '';
    when = '';
    repeat = '';
    await load();
  }

  async function done(id: string) {
    await apiPost(`/reminders/${id}/done`, {});
    await load();
  }

  async function snooze(id: string, mins: number) {
    const until = new Date(Date.now() + mins * 60000).toISOString();
    await apiPost(`/reminders/${id}/snooze`, { until });
    await load();
  }

  function dueLabel(r: DemoReminder): string {
    if (r.snoozed_until) return `Snoozed to ${new Date(r.snoozed_until).toLocaleString()}`;
    if (r.at) return new Date(r.at).toLocaleString();
    return 'No time';
  }
</script>

<main>
  <h1>Reminders</h1>
  <img class="mast" src="/art/module-reminders.webp" alt="" width="1536" height="1020" />
  {#if !live && stale}<p class="demo">Last live data. Start the server to refresh.</p>
  {:else if !live}<p class="demo">Sample data. Start the server to change it.</p>{/if}
  {#if loaded}
    <fieldset disabled={!live}>
      <section class="card">
        <h2>New</h2>
        <div class="form">
          <input bind:value={title} placeholder="Remind me to…" aria-label="Title" />
          <input bind:value={when} type="datetime-local" aria-label="Time" />
          <select bind:value={repeat} aria-label="Repeat">
            <option value="">Once</option>
            <option value="RRULE:FREQ=DAILY">Daily</option>
            <option value="RRULE:FREQ=WEEKLY">Weekly</option>
          </select>
          <button onclick={add}>Add</button>
        </div>
      </section>
      {#each items as r (r.id)}
        <section class="card">
          <h2>{r.title}</h2>
          <p class="age">{dueLabel(r)}{r.rrule ? ` · ${r.rrule.replace('RRULE:', '')}` : ''}</p>
          <div class="row">
            <button onclick={() => done(r.id)}>Done</button>
            <button onclick={() => snooze(r.id, 10)}>+10 min</button>
            <button onclick={() => snooze(r.id, 60)}>+1 h</button>
            <button onclick={() => snooze(r.id, 1440)}>Tomorrow</button>
          </div>
        </section>
      {/each}
      {#if items.length === 0}
        <section class="card calm">
          <img class="state" src="/art/empty-state.webp" alt="" width="768" height="768" />
          <h2>No reminders.</h2>
          <p>Add one above. Hearth nudges on time.</p>
        </section>
      {/if}
    </fieldset>
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
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
  }
  fieldset:disabled button,
  fieldset:disabled input,
  fieldset:disabled select {
    opacity: 0.55;
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
  .form {
    display: grid;
    gap: 0.5rem;
  }
  input,
  select {
    font: inherit;
    color: var(--paper);
    background: var(--ink);
    border: 1px solid var(--paper-dim);
    border-radius: 0.5rem;
    padding: 0.5rem 0.75rem;
  }
  button {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    border-radius: 0.5rem;
    padding: 0.5rem 1rem;
    font: inherit;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-top: 0.5rem;
  }
  .row button {
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--paper-dim);
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
  .state {
    display: block;
    width: 6rem;
    height: auto;
    border-radius: 0.75rem;
    margin-bottom: 0.5rem;
  }
</style>
