<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, apiPost } from '$lib/api';
  import { demoItems, demoRooms, type DemoItem } from '$lib/demo';

  let items = $state<DemoItem[]>([]);
  let rooms = $state<string[]>([]);
  let live = $state(true);
  let stale = $state(false);
  let loaded = $state(false);
  let q = $state('');
  let name = $state('');
  let room = $state('');
  let barcode = $state('');
  let note = $state('');
  let scanMsg = $state<string | null>(null);

  async function load() {
    const [a, b] = await Promise.all([
      apiGetOr<DemoItem[]>(
        q.trim() ? `/inventory?q=${encodeURIComponent(q.trim())}` : '/inventory',
        demoItems.filter((i) => matchesLocal(i, q))
      ),
      apiGetOr<string[]>('/inventory/rooms', demoRooms)
    ]);
    items = a.data;
    rooms = b.data;
    live = a.live && b.live;
    stale = a.stale || b.stale;
    loaded = true;
  }

  function matchesLocal(i: DemoItem, query: string): boolean {
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    return words.every((w) => i.name.toLowerCase().includes(w));
  }

  onMount(load);

  async function add() {
    if (!name.trim() || !room.trim()) return;
    await apiPost('/inventory', {
      name: name.trim(),
      room: room.trim(),
      barcode: barcode.trim() || null,
      note: note.trim() || null
    });
    name = '';
    room = '';
    barcode = '';
    note = '';
    await load();
  }

  async function scan() {
    // Phone camera scan where the browser allows it.
    const BD = (window as unknown as { BarcodeDetector?: new () => { detect(s: unknown): Promise<{ rawValue: string }[]> } }).BarcodeDetector;
    if (!BD) {
      scanMsg = 'This browser has no scan. Type the code by hand.';
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' } });
      const video = document.createElement('video');
      video.srcObject = stream;
      await video.play();
      const found = await new BD().detect(video);
      stream.getTracks().forEach((t) => t.stop());
      if (found.length > 0) {
        const code = found[0].rawValue;
        if (!live) {
          const hit = demoItems.find((i) => i.barcode === code);
          scanMsg = hit
            ? `Found: ${hit.name} lives in ${hit.room}.`
            : 'New code. Name it above to file it.';
          if (!hit) barcode = code;
          return;
        }
        const res = await apiGetOr<{ found: boolean; item?: DemoItem }>(
          `/inventory/by-barcode/${encodeURIComponent(code)}`,
          { found: false }
        );
        if (res.data.found && res.data.item) {
          q = res.data.item.name;
          await load();
          scanMsg = `Found: ${res.data.item.name} lives in ${res.data.item.room}.`;
        } else {
          barcode = code;
          scanMsg = 'New code. Name it above to file it.';
        }
      } else {
        scanMsg = 'No code seen. Retry in good light.';
      }
    } catch {
      scanMsg = 'Camera blocked. Type the code by hand.';
    }
  }
</script>

<main>
  <h1>Items</h1>
  <img class="mast" src="/art/module-items.webp" alt="" width="1536" height="1020" />
  {#if !live && stale}<p class="demo">Last live data. Start the server to refresh.</p>
  {:else if !live}<p class="demo">Sample data. Start the server to change it.</p>{/if}
  {#if loaded}
    <section class="card">
      <h2>Where is my…</h2>
      <div class="form">
        <input
          bind:value={q}
          oninput={load}
          placeholder="passport"
          aria-label="Search items"
        />
        <button onclick={scan}>Scan</button>
      </div>
      {#if scanMsg}<p class="age">{scanMsg}</p>{/if}
    </section>
    <fieldset disabled={!live}>
      <section class="card">
        <h2>File one</h2>
        <div class="form">
          <input bind:value={name} placeholder="Name" aria-label="Name" />
          <input bind:value={room} placeholder="Room" list="rooms" aria-label="Room" />
          <datalist id="rooms">
            {#each rooms as r}<option value={r}></option>{/each}
          </datalist>
          <input bind:value={barcode} placeholder="Code (from scan)" aria-label="Code" />
          <input bind:value={note} placeholder="Note" aria-label="Note" />
          <button onclick={add}>File it</button>
        </div>
      </section>
    </fieldset>
    {#each items as i (i.id)}
      <section class="card">
        <h2>{i.name}</h2>
        <p class="age">{i.room}{i.note ? ` · ${i.note}` : ''}</p>
      </section>
    {/each}
    {#if items.length === 0}
      <section class="card calm">
        <img class="state" src="/art/empty-state.webp" alt="" width="768" height="768" />
        <h2>Empty here.</h2>
        <p>File your first item above.</p>
      </section>
    {/if}
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
  fieldset:disabled input {
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
  input {
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
