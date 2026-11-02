<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, apiPost, ageLabel } from '$lib/api';
  import { quiet24h } from '$lib/needs-you';
  import {
    demoThreads,
    demoAskQueue,
    type ThreadBoard,
    type ThreadFlow,
    type AskQueue
  } from '$lib/demo';

  type Lane = 'need' | 'building' | 'paused' | 'review' | 'waiting';
  const lanes: { id: Lane; title: string }[] = [
    { id: 'need', title: 'Needs answer' },
    { id: 'building', title: 'Building' },
    { id: 'paused', title: 'Paused' },
    { id: 'review', title: 'Ready for review' },
    { id: 'waiting', title: 'Waits for next task' }
  ];

  let board = $state<ThreadBoard>(demoThreads);
  let queue = $state<AskQueue>(demoAskQueue);
  let live = $state(true);
  let staleNote = $state('');
  let loaded = $state(false);
  let answer = $state('');
  let forThread = $state('');
  let saveNote = $state('');
  let voiceOn = $state(false);

  const atLane = (l: Lane) => board.threads.filter((t) => t.flow === l);
  const doneThreads = () => board.threads.filter((t) => t.flow === 'review');
  const pip = (t: ThreadFlow) =>
    t.flow === 'building'
      ? 'var(--ok)'
      : t.flow === 'need'
        ? 'var(--warn)'
        : t.flow === 'paused'
          ? 'var(--bad)'
          : 'var(--paper-dim)';

  // Voice fills the box. Typed text always works. Nothing sends.
  function voiceFill() {
    const w = window as unknown as {
      SpeechRecognition?: new () => VoiceRec;
      webkitSpeechRecognition?: new () => VoiceRec;
    };
    const SR = w.SpeechRecognition ?? w.webkitSpeechRecognition;
    if (!SR) {
      saveNote = 'Voice needs Chrome. Type instead.';
      return;
    }
    type VoiceRec = {
      lang: string;
      onresult: ((e: { results: { transcript: string }[][] }) => void) | null;
      onend: (() => void) | null;
      start: () => void;
    };
    const rec = new SR();
    rec.lang = 'en-US';
    voiceOn = true;
    rec.onresult = (e) => {
      answer = e.results[0][0].transcript;
    };
    rec.onend = () => {
      voiceOn = false;
    };
    rec.start();
  }

  async function saveAnswer() {
    saveNote = '';
    try {
      const res = (await apiPost('/watch/ask-queue', {
        thread: forThread || null,
        text: answer
      })) as { saved: boolean; at: string };
      if (res.saved) {
        answer = '';
        saveNote = `Saved at ${res.at}. Nothing sends yet.`;
        const q = await apiGetOr<AskQueue>('/watch/ask-queue', demoAskQueue);
        queue = q.data;
      }
    } catch (e) {
      saveNote = (e as { message: string }).message ?? 'Save failed. Retry soon.';
    }
  }

  // The baked snapshot fills gaps when the API stays quiet.
  async function loadFeed(): Promise<ThreadBoard | null> {
    try {
      const res = await fetch('/threads-status.json');
      if (!res.ok) return null;
      const feed = (await res.json()) as Partial<ThreadBoard>;
      if (!Array.isArray(feed.threads) || !Array.isArray(feed.needs_answer)) return null;
      return feed as ThreadBoard;
    } catch {
      return null;
    }
  }

  // Live API first. Then the baked snapshot. Demo fills last gaps.
  // A snapshot never wins over fresh data: threads move every minute.
  onMount(async () => {
    const b = await apiGetOr<ThreadBoard>('/watch/threads', demoThreads);
    const feed = await loadFeed();
    const q = await apiGetOr<AskQueue>('/watch/ask-queue', demoAskQueue);
    queue = q.data;
    if (b.live) {
      board = b.data;
      live = true;
      staleNote = '';
    } else if (feed) {
      board = feed;
      live = false;
      staleNote = feed.generated_at
        ? `Snapshot from ${ageLabel(new Date(feed.generated_at))}.`
        : 'Baked snapshot. No live link.';
    } else {
      board = b.data;
      live = false;
      staleNote = b.stale ? `Last known from ${ageLabel(b.at)}.` : '';
    }
    const done = doneThreads();
    if (done.length > 0) forThread = done[0].name;
    loaded = true;
    // Lane deep link (?lane=need). Counts elsewhere open exact lanes.
    const lane = new URLSearchParams(location.search).get('lane') ?? '';
    if (lanes.some((l) => l.id === lane)) {
      requestAnimationFrame(() => {
        document.getElementById(`lane-${lane}`)?.scrollIntoView();
      });
    }
  });
</script>

<main>
  <h1>Threads</h1>
  {#if !live && !staleNote}<p class="demo">Sample data.</p>{/if}
  {#if staleNote}<p class="demo">{staleNote}</p>{/if}
  {#if loaded}
    {#if board.needs_answer.length > 0}
      <section class="card ask" aria-label="Needs the owner answer">
        <h2>Needs the owner answer</h2>
        {#each board.needs_answer as n}
          <div class="row">
            <p><span class="pip" style="background: var(--warn)"></span>{n.thread}</p>
            <p class="q">{n.question}</p>
            <p class="age">{n.recommendation ?? 'No recommendation yet.'}</p>
          </div>
        {/each}
      </section>
    {/if}
    <div class="board">
      {#each lanes as l}
        <section class="lane" id={`lane-${l.id}`} aria-label={l.title}>
          <h2>{l.title} <span class="num">{atLane(l.id).length}</span></h2>
          {#each atLane(l.id) as t (t.pane)}
            <div class="chip">
              <p class="name"><span class="pip" style="background: {pip(t)}"></span>{t.name}</p>
              <p class="age">{t.state}{t.changed_at ? ` · since ${t.changed_at}` : ''}</p>
              {#if quiet24h(t.changed_at)}<p class="age quiet">Quiet 24 h.</p>{/if}
              <p class="line">{t.line}</p>
              <p class="age">{t.stage ?? 'No stage report yet.'}</p>
            </div>
          {/each}
        </section>
      {/each}
    </div>
    {#if doneThreads().length > 0}
      <section class="card" aria-label="What should we build next">
        <h2>What should we build next?</h2>
        <p class="age">Type or speak. The answer waits in a queue. Nothing sends to any thread yet.</p>
        <div class="askrow">
          <select bind:value={forThread} aria-label="Thread for the answer">
            <option value="">Next build</option>
            {#each doneThreads() as t (t.pane)}
              <option value={t.name}>{t.name}</option>
            {/each}
          </select>
          <input bind:value={answer} maxlength="2000" placeholder="Name the next build." aria-label="Next build answer" />
        </div>
        <div class="askrow">
          <button onclick={saveAnswer} disabled={answer.trim() === ''}>Save answer</button>
          <button class="ghost" onclick={voiceFill} disabled={voiceOn}>
            {voiceOn ? 'Listening.' : 'Speak it'}
          </button>
        </div>
        {#if saveNote}<p class="demo">{saveNote}</p>{/if}
        {#if queue.entries.length > 0}
          <h3>Waiting answers</h3>
          {#each queue.entries as e}
            <p class="qline">{e.at} · {e.thread ?? 'Next build'} · {e.text}</p>
          {/each}
        {/if}
      </section>
    {/if}
    <p class="age foot">
      Read-only except the answer box. {board.log_at
        ? `Newest log line: ${board.log_at}.`
        : 'No log yet.'}
    </p>
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
  h3 {
    font-size: var(--text-sm);
    color: var(--paper-dim);
    margin: 1rem 0 0.25rem;
  }
  .age {
    color: var(--paper-dim);
    font-size: var(--text-sm);
  }
  .age.quiet {
    color: var(--warn);
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
    margin: 0 0 0.5rem;
    color: var(--paper-dim);
  }
  .num {
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
  .chip .line {
    margin: 0.35rem 0 0;
    font-size: var(--text-sm);
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
  .pip {
    display: inline-block;
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    margin-right: 0.5rem;
  }
  .ask .row {
    border-top: 1px solid var(--ink);
    padding-top: 0.5rem;
    margin-top: 0.5rem;
  }
  .ask .row:first-of-type {
    border-top: none;
    padding-top: 0;
  }
  .q {
    color: var(--warn);
  }
  .qline {
    font-size: var(--text-sm);
  }
  .askrow {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }
  select,
  input {
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--ink);
    border-radius: 0.75rem;
    padding: 0.6rem 0.8rem;
    font-size: var(--text-sm);
    min-width: 0;
  }
  input {
    flex: 1;
  }
  button {
    background: var(--accent);
    color: var(--ink);
    border: none;
    border-radius: 0.75rem;
    padding: 0.6rem 1rem;
    font-size: var(--text-sm);
  }
  button:disabled {
    opacity: 0.45;
  }
  button.ghost {
    background: var(--ink);
    color: var(--paper);
  }
  .foot {
    margin-top: 1rem;
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
