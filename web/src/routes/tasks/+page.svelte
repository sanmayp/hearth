<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, apiPost } from '$lib/api';
  import { demoTasks, type DemoTask } from '$lib/demo';

  let tasks = $state<DemoTask[]>([]);
  let live = $state(true);
  let stale = $state(false);
  let loaded = $state(false);
  let title = $state('');
  let list = $state<'inbox' | 'today' | 'someday'>('inbox');
  let undone = $state<string | null>(null);
  let justDone = $state<string | null>(null);
  let canVoice = $state(false);
  let listening = $state(false);
  let heard = $state('');

  const lists = [
    { key: 'inbox', label: 'Inbox' },
    { key: 'today', label: 'Now' },
    { key: 'someday', label: 'Later' }
  ] as const;

  async function load() {
    const res = await apiGetOr<DemoTask[]>('/tasks', demoTasks);
    tasks = res.data;
    live = res.live;
    stale = res.stale;
    loaded = true;
  }

  onMount(() => {
    const w = window as unknown as Record<string, unknown>;
    canVoice = typeof w.SpeechRecognition === 'function' || typeof w.webkitSpeechRecognition === 'function';
    load();
  });

  type Rec = {
    lang: string;
    interimResults: boolean;
    onresult: ((e: { results: { 0: { 0: { transcript: string } } } }) => void) | null;
    onerror: (() => void) | null;
    onend: (() => void) | null;
    start: () => void;
  };

  function voice() {
    const w = window as unknown as Record<string, new () => Rec>;
    const Make = w.SpeechRecognition ?? w.webkitSpeechRecognition;
    if (!Make) return;
    const rec = new Make();
    rec.lang = navigator.language || 'en-US';
    rec.interimResults = false;
    listening = true;
    heard = '';
    rec.onresult = (e) => {
      const said = e.results[0][0].transcript.trim();
      listening = false;
      if (said) fileVoice(said);
    };
    rec.onerror = () => {
      listening = false;
      heard = 'Heard nothing. Retry in quiet.';
    };
    rec.onend = () => (listening = false);
    rec.start();
  }

  async function fileVoice(said: string) {
    const low = said.toLowerCase();
    if (low === 'remind me' || low.startsWith('remind me ')) {
      let rest = said.slice('remind me'.length).trim();
      if (rest.toLowerCase().startsWith('to ')) rest = rest.slice(3).trim();
      const label = rest || said;
      await apiPost('/reminders', { title: label, at: null, rrule: null });
      heard = `Heard: ${said}. Filed as reminder: ${label}.`;
    } else if (low === 'note' || low.startsWith('note ')) {
      const rest = said.slice('note'.length).trim() || said;
      await apiPost('/tasks', { title: rest, list: 'inbox' });
      heard = `Heard: ${said}. Filed in Inbox: ${rest}.`;
    } else {
      await apiPost('/tasks', { title: said, list: 'today' });
      heard = `Heard: ${said}. Filed for Now: ${said}.`;
    }
    await load();
  }

  async function add() {
    if (!title.trim()) return;
    await apiPost('/tasks', { title: title.trim(), list });
    title = '';
    await load();
  }

  async function done(id: string) {
    await apiPost(`/tasks/${id}/done`, {});
    justDone = id;
    undone = id;
    setTimeout(() => {
      justDone = null;
      undone = null;
    }, 6000);
    await load();
  }

  async function undo() {
    if (!undone) return;
    await apiPost(`/tasks/${undone}/undo`, {});
    undone = null;
    await load();
  }

  async function move(id: string, to: string) {
    await apiPost(`/tasks/${id}/move`, { list: to });
    await load();
  }

  async function split(id: string) {
    await apiPost(`/tasks/${id}/split`, {});
    await load();
  }

  async function stepDone(id: string, n: number) {
    await apiPost(`/tasks/${id}/steps/${n}/done`, {});
    await load();
  }
</script>

<main>
  <h1>Tasks</h1>
  <img class="mast" src="/art/module-tasks.webp" alt="" width="1536" height="1020" />
  {#if !live && stale}<p class="demo">Last live data. Start the server to refresh.</p>
  {:else if !live}<p class="demo">Sample data. Start the server to change it.</p>{/if}
  {#if loaded}
    <fieldset disabled={!live}>
      <section class="card">
        <h2>New</h2>
        <div class="form">
          <input bind:value={title} placeholder="Do the thing…" aria-label="Title" />
          <select bind:value={list} aria-label="List">
            <option value="inbox">Inbox</option>
            <option value="today">Now</option>
            <option value="someday">Later</option>
          </select>
          <button onclick={add}>Add</button>
          {#if canVoice}
            <button class="speak" onclick={voice}>{listening ? 'Hearing…' : 'Speak it'}</button>
          {/if}
        </div>
        {#if heard}<p class="age">{heard}</p>{/if}
      </section>
      {#each lists as l}
        <h2 class="listhead">{l.label}</h2>
        {#each tasks.filter((t) => t.list === l.key) as t (t.id)}
          <section class="card" class:done={justDone === t.id}>
            <div class="head">
              <button class="check" onclick={() => done(t.id)} aria-label="Done">✓</button>
              <h3>{t.title}</h3>
            </div>
            {#if t.steps.length > 0}
              <ol>
                {#each t.steps as s, n}
                  <li class:struck={s.done}>
                    {#if s.done}✓ {s.title}{:else}<button
                        class="step"
                        onclick={() => stepDone(t.id, n)}>{s.title}</button
                      >{/if}
                  </li>
                {/each}
              </ol>
            {:else}
              <button class="step" onclick={() => split(t.id)}>Cut into steps</button>
            {/if}
            <div class="row">
              {#each lists.filter((x) => x.key !== t.list) as m}
                <button onclick={() => move(t.id, m.key)}>→ {m.label}</button>
              {/each}
            </div>
          </section>
        {/each}
      {/each}
    </fieldset>
  {:else}
    <section class="card skeleton" aria-busy="true">
      <div class="bar"></div>
      <div class="bar short"></div>
    </section>
  {/if}
  {#if undone}
    <div class="toast" role="status">
      <span>Done.</span>
      <button onclick={undo}>Undo</button>
    </div>
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
  .listhead {
    font-size: var(--text-sm);
    color: var(--paper-dim);
    margin: 1.5rem 0 0;
  }
  .card {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1.25rem 1.5rem;
    margin-top: 0.75rem;
  }
  .card h2 {
    font-size: var(--text-md);
    margin: 0 0 0.5rem;
  }
  .card.done {
    animation: tick var(--dur-med) var(--ease-out);
  }
  @keyframes tick {
    0% {
      border: 2px solid var(--accent);
    }
    100% {
      border: 2px solid transparent;
    }
  }
  .head {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }
  .head h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  .check {
    flex: none;
    width: 2rem;
    height: 2rem;
    border-radius: 999px;
  }
  ol {
    margin: 0.5rem 0;
    padding-left: 1.25rem;
  }
  .struck {
    text-decoration: line-through;
    color: var(--paper-dim);
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
    margin-top: 0.5rem;
  }
  .row button,
  .step {
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--paper-dim);
  }
  .speak {
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--accent);
  }
  .step {
    margin-top: 0.5rem;
  }
  .toast {
    position: fixed;
    left: 50%;
    bottom: 4.5rem;
    transform: translateX(-50%);
    display: flex;
    gap: 1rem;
    align-items: center;
    background: var(--ink-raise);
    border: 1px solid var(--accent);
    border-radius: 0.75rem;
    padding: 0.75rem 1rem;
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
  @media (prefers-reduced-motion: reduce) {
    .card.done {
      animation: none;
    }
  }
</style>
