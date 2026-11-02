<script lang="ts">
  import { copyText } from '$lib/copy';
  import { screenRouteFor, type YourMoveItem } from '$lib/features';

  type ToggleResult = { done: boolean; error: string | null };

  let {
    item,
    done,
    doneAt,
    busy,
    onToggle
  }: {
    item: YourMoveItem;
    done: boolean;
    doneAt: string;
    busy: boolean;
    onToggle: (featureId: string, stepIndex: number, want: boolean) => Promise<ToggleResult>;
  } = $props();

  let expanded = $state(false);
  let asking = $state(false);
  let localErr = $state('');
  let copied = $state('');

  const goHref = () => screenRouteFor(item.where);

  async function copyCmd(): Promise<void> {
    copied = (await copyText(item.cmd)) ? 'Copied.' : 'Copy failed. Type it by hand.';
  }

  async function confirmToggle(): Promise<void> {
    asking = false;
    localErr = '';
    const res = await onToggle(item.featureId, item.stepIndex, !done);
    if (res.error) {
      localErr = `Failed: ${res.error}`;
    } else {
      expanded = false;
    }
  }
</script>

<div class="srow">
  <div class="stop">
    <button
      type="button"
      class="scheck"
      disabled={busy}
      aria-label={done ? 'Reopen this step' : 'Mark this step done'}
      onclick={() => {
        asking = true;
        localErr = '';
      }}
    >
      {done ? '☑' : '☐'}
    </button>
    <button type="button" class="stextbtn" onclick={() => (expanded = !expanded)}>
      <span class="sfeat">{item.featureName}</span>
      <span class="stext">{item.text}</span>
      {#if done && doneAt}<span class="sdone">done {doneAt}</span>{/if}
      <span class="shint">Tap to see how</span>
    </button>
  </div>
  {#if asking}
    <div class="sask">
      <span>{done ? 'Reopen?' : 'Mark done?'}</span>
      <button type="button" class="sbtn" disabled={busy} onclick={() => void confirmToggle()}>
        Yes
      </button>
      <button type="button" class="sbtn ghost" disabled={busy} onclick={() => (asking = false)}>
        No
      </button>
    </div>
  {/if}
  {#if localErr}<p class="serr">{localErr}</p>{/if}
  {#if expanded}
    <div class="sdetail">
      {#if item.where}<p class="swhere">{item.where}</p>{/if}
      {#if item.how}<p class="show">{item.how}</p>{/if}
      {#if item.cmd}
        <div class="scmdrow">
          <code>{item.cmd}</code>
          <button type="button" class="sbtn" onclick={() => void copyCmd()}>
            {copied ? copied : 'Copy'}
          </button>
        </div>
      {/if}
      {#if goHref()}<a class="sbtn go" href={goHref()}>Go</a>{/if}
    </div>
  {/if}
</div>

<style>
  .srow {
    background: var(--ink-raise);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--ink));
    border-radius: 0.85rem;
    padding: 0.65rem 0.85rem;
  }
  .stop {
    display: flex;
    align-items: flex-start;
    gap: 0.55rem;
  }
  .scheck {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    font-size: var(--text-md);
    line-height: 1.2;
    cursor: pointer;
    flex: none;
  }
  .scheck:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .stextbtn {
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    color: inherit;
    font: inherit;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    min-width: 0;
  }
  .sfeat {
    font-size: var(--text-xs);
    color: var(--paper-dim);
    font-weight: 600;
  }
  .stext {
    font-size: var(--text-sm);
    line-height: 1.3;
  }
  .sdone {
    font-size: var(--text-xs);
    color: var(--ok);
  }
  .shint {
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .sask {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    font-size: var(--text-sm);
  }
  .sbtn {
    background: var(--accent);
    color: var(--accent-ink);
    border: none;
    border-radius: 0.6rem;
    padding: 0.3rem 0.8rem;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    text-decoration: none;
    display: inline-block;
  }
  .sbtn.ghost {
    background: var(--ink);
    color: var(--paper);
  }
  .sbtn:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .serr {
    margin: 0.4rem 0 0;
    font-size: var(--text-sm);
    color: var(--bad);
  }
  .sdetail {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid color-mix(in srgb, var(--paper-dim) 25%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .swhere,
  .show {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--paper-dim);
    line-height: 1.3;
  }
  .scmdrow {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .scmdrow code {
    background: var(--ink);
    color: var(--paper);
    border-radius: 0.6rem;
    padding: 0.35rem 0.65rem;
    font-size: var(--text-sm);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .sdetail .go {
    align-self: flex-start;
    margin-top: 0.15rem;
  }
</style>
