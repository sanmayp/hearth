<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { apiGetOr, apiPost, saveStash, type ApiError } from '$lib/api';
  import {
    demoFeatures,
    todayIso,
    yourMoveList,
    type FeaturesLedger
  } from '$lib/features';
  import StepRow from '$lib/StepRow.svelte';

  let ledger = $state<FeaturesLedger>(demoFeatures);
  let busyId = $state<string | null>(null);
  let undoOffer = $state<{ id: string; index: number; text: string } | null>(null);
  let undoTimer: ReturnType<typeof setTimeout> | null = null;
  let stepNotice = $state('');

  const moves = () => yourMoveList(ledger.features);

  function applyLocal(id: string, index: number, date: string, done: boolean): void {
    ledger = {
      ...ledger,
      features: ledger.features.map((f) => {
        if (f.id !== id || !Array.isArray(f.steps)) return f;
        return {
          ...f,
          steps: f.steps.map((s, idx) =>
            idx !== index || s.owner !== 'you'
              ? s
              : { ...s, done, done_at: done ? date : '' }
          )
        };
      })
    };
    saveStash('/watch/features', ledger);
  }

  async function toggle(
    id: string,
    index: number,
    want: boolean
  ): Promise<{ done: boolean; error: string | null }> {
    busyId = `${id}:step:${index}`;
    const today = todayIso();
    try {
      const res = await apiPost<{ done?: boolean; done_at?: string }>(
        `/watch/features/${encodeURIComponent(id)}/step`,
        { index, done: want }
      );
      const nextDone = res.done ?? want;
      applyLocal(id, index, res.done_at ?? (nextDone ? today : ''), nextDone);
      return { done: nextDone, error: null };
    } catch (err) {
      const apiErr = err as ApiError;
      const offline = apiErr?.status === undefined || apiErr.status === 0;
      applyLocal(id, index, today, want);
      return {
        done: want,
        error: offline ? null : (apiErr?.message ?? 'Step is unchanged. Retry.')
      };
    } finally {
      busyId = null;
    }
  }

  async function onStepToggle(
    id: string,
    index: number,
    want: boolean
  ): Promise<{ done: boolean; error: string | null }> {
    const r = await toggle(id, index, want);
    if (undoTimer) clearTimeout(undoTimer);
    if (!r.error && r.done) {
      const text = ledger.features.find((f) => f.id === id)?.steps?.[index]?.text ?? 'step';
      undoOffer = { id, index, text };
      undoTimer = setTimeout(() => {
        undoOffer = null;
        undoTimer = null;
      }, 5000);
    } else {
      undoOffer = null;
      undoTimer = null;
    }
    stepNotice = !r.error && !r.done ? 'Reopened.' : '';
    return r;
  }

  async function undoStep(): Promise<void> {
    const o = undoOffer;
    if (undoTimer) clearTimeout(undoTimer);
    undoOffer = null;
    undoTimer = null;
    if (o) await toggle(o.id, o.index, false);
  }

  onDestroy(() => {
    if (undoTimer) clearTimeout(undoTimer);
  });

  onMount(async () => {
    const b = await apiGetOr<FeaturesLedger>('/watch/features', demoFeatures);
    if (b.live && Array.isArray(b.data?.features)) {
      ledger = b.data;
      return;
    }
    try {
      const res = await fetch('/features.json');
      if (res.ok) {
        const snap = (await res.json()) as Partial<FeaturesLedger>;
        if (Array.isArray(snap.features)) {
          ledger = {
            updated: snap.updated ?? null,
            features: snap.features,
            generated_at: snap.generated_at
          };
          return;
        }
      }
    } catch {
      // Fall back to demoFeatures.
    }
  });
</script>

<section class="ymstrip" aria-label="Your move">
  <div class="ymhead">
    <h2>Your move <span class="ymcount">{moves().length}</span></h2>
    <a class="ymlink" href="/features">Open Features</a>
  </div>
  {#if moves().length === 0}
    <p class="ymempty">No steps wait on you.</p>
  {:else}
    <p class="ymmean">{moves().length} steps wait on you.</p>
    <p class="ymhint">Tap a step to see how.</p>
    {#if undoOffer}
      <p class="ymundo" role="status">
        Marked done: {undoOffer.text}.
        <button type="button" class="ymundobtn" onclick={() => void undoStep()}>Undo</button>
      </p>
    {/if}
    {#if stepNotice && !undoOffer}
      <p class="ymnotice" role="status">{stepNotice}</p>
    {/if}
    <ul class="ymlist">
      {#each moves() as m (`strip-${m.featureId}-${m.stepIndex}`)}
        <li>
          <StepRow
            item={m}
            done={false}
            doneAt=""
            busy={busyId === `${m.featureId}:step:${m.stepIndex}`}
            onToggle={onStepToggle}
          />
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .ymstrip {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 0.9rem 1rem;
  }
  .ymhead {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.45rem;
  }
  h2 {
    font-size: var(--text-md);
    margin: 0;
  }
  .ymcount {
    font-size: var(--text-xs);
    color: var(--paper-dim);
    font-variant-numeric: var(--nums);
    margin-left: 0.25rem;
  }
  .ymlink {
    color: var(--accent);
    font-size: var(--text-xs);
    text-decoration: underline;
  }
  .ymempty {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .ymmean {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .ymhint {
    margin: 0.15rem 0 0.45rem;
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .ymundo {
    margin: 0 0 0.45rem;
    font-size: var(--text-xs);
    color: var(--ok);
  }
  .ymnotice {
    margin: 0 0 0.45rem;
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .ymundobtn {
    background: none;
    border: none;
    padding: 0 0 0 0.5rem;
    color: var(--paper);
    font-size: var(--text-xs);
    text-decoration: underline;
    cursor: pointer;
  }
  .ymlist {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
</style>
