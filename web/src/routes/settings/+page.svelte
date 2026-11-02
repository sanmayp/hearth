<script lang="ts">
  import { onMount } from 'svelte';
  import { loadToken, saveToken } from '$lib/api';
  import { loadBridgeKey, saveBridgeKey, loadBridgePort, saveBridgePort } from '$lib/bridge';
  import { loadUpdateState, runUpdateCheck, runUpdateInstall, type UpdateState } from '$lib/update';

  let token = $state('');
  let saved = $state(false);
  let bridgeKey = $state('');
  let bridgeSaved = $state(false);
  let bridgePort = $state('');
  let upd = $state<UpdateState>(loadUpdateState());
  let busy = $state('');

  onMount(() => {
    token = loadToken();
    saved = token.length > 0;
    bridgeKey = loadBridgeKey();
    bridgeSaved = bridgeKey.length > 0;
    bridgePort = loadBridgePort();
    upd = loadUpdateState();
  });

  function keep() {
    saveToken(token.trim());
    saved = token.trim().length > 0;
  }

  function drop() {
    saveToken('');
    token = '';
    saved = false;
  }

  function keepBridge() {
    saveBridgeKey(bridgeKey.trim());
    bridgeSaved = bridgeKey.trim().length > 0;
  }

  function dropBridge() {
    saveBridgeKey('');
    bridgeKey = '';
    bridgeSaved = false;
  }

  function keepPort() {
    saveBridgePort(bridgePort.trim());
    bridgePort = loadBridgePort();
  }

  async function check() {
    busy = 'Checking for updates.';
    upd = await runUpdateCheck();
    busy = '';
  }

  async function update() {
    busy = 'Starting download.';
    try {
      busy = await runUpdateInstall((msg) => (busy = msg));
    } catch (e) {
      busy = String(e);
    }
  }
</script>

<main>
  <h1>Settings</h1>
  <section class="card">
    <h2>Server key</h2>
    <p>Paste the key once. It stays on this device. Hearth asks for it on every call.</p>
    <div class="row">
      <input
        type="password"
        placeholder="Paste key"
        aria-label="Server key"
        bind:value={token}
        autocomplete="off"
      />
      <button onclick={keep}>Keep</button>
      {#if saved}
        <button class="ghost" onclick={drop}>Forget</button>
      {/if}
    </div>
    <p class="age">{saved ? 'Key kept on this device.' : 'No key kept.'}</p>
  </section>
  <section class="card">
    <h2>Bridge key</h2>
    <p>Controls agent sessions from this phone. It stays on this device. It never ships in the app.</p>
    <div class="row">
      <input
        type="password"
        placeholder="Paste bridge key"
        aria-label="Bridge key"
        bind:value={bridgeKey}
        autocomplete="off"
      />
      <button onclick={keepBridge}>Keep</button>
      {#if bridgeSaved}
        <button class="ghost" onclick={dropBridge}>Forget</button>
      {/if}
    </div>
    <p class="age">{bridgeSaved ? 'Bridge key kept on this device.' : 'No bridge key kept.'}</p>
    <div class="row">
      <input
        inputmode="numeric"
        placeholder="Bridge port"
        aria-label="Bridge port"
        bind:value={bridgePort}
        autocomplete="off"
      />
      <button onclick={keepPort}>Keep</button>
    </div>
    <p class="age">Bridge host matches the server host. Default port is 8449.</p>
  </section>
  <section class="card">
    <h2>App update</h2>
    {#if upd.status === 'available'}
      <p>Update available: {upd.version}.</p>
      {#if upd.notes}<p class="age">{upd.notes}</p>{/if}
      <div class="row">
        <button onclick={update}>Update</button>
      </div>
    {:else if upd.status === 'current'}
      <p>This app is current{upd.version ? ` (${upd.version})` : ''}.</p>
    {:else if upd.status === 'error'}
      <p>Update check failed.</p>
      <p class="age">{upd.detail}</p>
    {:else}
      <p>No update check yet.</p>
      {#if upd.detail}<p class="age">{upd.detail}</p>{/if}
    {/if}
    {#if busy}<p class="age">{busy}</p>{/if}
    <div class="row">
      <button class="ghost" onclick={check}>Check for update</button>
    </div>
  </section>
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
  .row {
    display: grid;
    gap: 0.5rem;
    grid-template-columns: 1fr auto auto;
    margin-top: 0.75rem;
  }
  input,
  button {
    font: inherit;
    border-radius: 0.6rem;
    padding: 0.55rem 0.8rem;
  }
  input {
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--paper-dim);
    min-width: 0;
  }
  button {
    background: var(--accent);
    color: var(--accent-ink);
    border: none;
    font-weight: 600;
  }
  @media (max-width: 30rem) {
    .row {
      grid-template-columns: 1fr;
    }
  }
  .ghost {
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--paper-dim);
  }
</style>
