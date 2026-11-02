<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, apiPost } from '$lib/api';
  import { demoCheckins, type DemoCheckin } from '$lib/demo';

  type Help = { line: string; contact_name: string; contact_phone: string; note: string };

  // Client copy of the server crisis list. Same words, same order.
  // It guards sample mode. The server guards live mode.
  const CRISIS_WORDS = [
    'kill myself', 'killing myself', 'suicide', 'suicidal', 'self-harm',
    'self harm', 'hurt myself', 'harm myself', 'end my life', 'end it all',
    'want to die', 'wish i were dead', 'no reason to live'
  ];

  let past = $state<DemoCheckin[]>([]);
  let live = $state(true);
  let stale = $state(false);
  let loaded = $state(false);
  let mood = $state(0);
  let energy = $state(0);
  let note = $state('');
  let saved = $state(false);
  let crisis = $state<Help | null>(null);

  async function load() {
    const res = await apiGetOr<DemoCheckin[]>('/health/checkins?days=7', demoCheckins);
    past = res.data;
    live = res.live;
    stale = res.stale;
    loaded = true;
  }

  onMount(load);

  function localCrisis(text: string): boolean {
    const low = text.toLowerCase();
    return CRISIS_WORDS.some((w) => low.includes(w));
  }

  async function send() {
    if (mood < 1 || energy < 1) return;
    // Crisis check first, in both modes. It never waits on the server.
    if (note.trim() && localCrisis(note)) {
      crisis = {
        line: '988',
        contact_name: 'Your emergency contact',
        contact_phone: '555-010-0000',
        note: 'Stop here. Call or text now. You matter.'
      };
      return;
    }
    if (!live) {
      // Sample mode: keep it local, say so.
      past = [
        { id: 'local', day: new Date().toISOString().slice(0, 10), payload: { mood, energy, note: note.trim() || null }, at: new Date().toISOString() },
        ...past
      ];
    } else {
      const res = await apiPost<{ ok: boolean; crisis: boolean; help?: Help }>('/health/checkin', {
        mood,
        energy,
        note: note.trim() || null
      });
      if (res.crisis && res.help) {
        crisis = res.help;
        return;
      }
      await load();
    }
    mood = 0;
    energy = 0;
    note = '';
    saved = true;
    setTimeout(() => (saved = false), 4000);
  }

  const faces = ['1', '2', '3', '4', '5'];
</script>

<main>
  <h1>Health</h1>
  <img class="mast" src="/art/module-health.webp" alt="" width="1536" height="1020" />
  {#if crisis}
    <section class="card crisis" role="alert">
      <h2>Pause. Get help now.</h2>
      <p>{crisis.note}</p>
      <p class="big num">Call or text {crisis.line}</p>
      <p class="big num">{crisis.contact_name}: {crisis.contact_phone}</p>
    </section>
  {:else if loaded}
    {#if !live && stale}<p class="demo">Last live data. New check-ins stay on this device.</p>
    {:else if !live}<p class="demo">Sample data. Check-ins stay on this device until the server runs.</p>{/if}
    <section class="card">
      <h2>Check-in (10 sec)</h2>
      <p class="age">Mood</p>
      <div class="row" role="radiogroup" aria-label="Mood">
        {#each faces as f, i}
          <button class:on={mood === i + 1} onclick={() => (mood = i + 1)} aria-label="Mood {f}">{f}</button>
        {/each}
      </div>
      <p class="age">Energy</p>
      <div class="row" role="radiogroup" aria-label="Energy">
        {#each faces as f, i}
          <button class:on={energy === i + 1} onclick={() => (energy = i + 1)} aria-label="Energy {f}">{f}</button>
        {/each}
      </div>
      <input bind:value={note} placeholder="Note (sealed, yours only)" aria-label="Note" />
      <button class="send" onclick={send} disabled={mood < 1 || energy < 1}>Save</button>
      {#if saved}<p class="age">{live ? 'Saved. Sealed on this server.' : 'Saved here only. It leaves when the server runs.'}</p>{/if}
    </section>
    {#each past as c (c.id)}
      <section class="card">
        <h2 class="num">{c.day}</h2>
        <p class="age num">Mood {c.payload.mood} · Energy {c.payload.energy}</p>
        {#if c.payload.note}<p>{c.payload.note}</p>{/if}
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
  .crisis {
    border: 2px solid var(--bad);
  }
  .big {
    font-size: var(--text-lg);
  }
  .row {
    display: flex;
    gap: 0.5rem;
    margin: 0.25rem 0 0.75rem;
  }
  .row button {
    flex: 1;
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--paper-dim);
  }
  .row button.on {
    background: var(--accent);
    color: var(--accent-ink);
    border-color: var(--accent);
  }
  input {
    width: 100%;
    font: inherit;
    color: var(--paper);
    background: var(--ink);
    border: 1px solid var(--paper-dim);
    border-radius: 0.5rem;
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.75rem;
  }
  button {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    border-radius: 0.5rem;
    padding: 0.5rem 1rem;
    font: inherit;
  }
  button:disabled {
    opacity: 0.4;
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
