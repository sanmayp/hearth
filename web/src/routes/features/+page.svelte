<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { apiGetOr, apiPost, ageLabel, saveStash, type ApiError } from '$lib/api';
  import { pullToRefresh, updatedStamp } from '$lib/refresh';
  import StepRow from '$lib/StepRow.svelte';
  import {
    attentionList,
    claimedLabel,
    demoFeatures,
    doneList,
    groupByArea,
    inProgressList,
    isQuiet24h,
    ownerConfirmLabel,
    ownerConfirmTone,
    proofText,
    splitWhereLinks,
    stageTone,
    stepProgressLabel,
    todayIso,
    yourMoveList,
    type FeatureRow,
    type FeaturesLedger
  } from '$lib/features';

  let ledger = $state<FeaturesLedger>(demoFeatures);
  let live = $state(true);
  let staleNote = $state('');
  let loaded = $state(false);
  let updatedAt = $state<Date | null>(null);
  let refreshing = $state(false);
  let busyId = $state<string | null>(null);
  let markNotice = $state<Record<string, string>>({});
  type UndoOffer = { id: string; index: number; text: string };
  let undoOffer = $state<UndoOffer | null>(null);
  let undoTimer: ReturnType<typeof setTimeout> | null = null;
  let moveNotice = $state('');

  function offerUndo(id: string, index: number, text: string): void {
    if (undoTimer) clearTimeout(undoTimer);
    undoOffer = { id, index, text };
    undoTimer = setTimeout(() => {
      undoOffer = null;
      undoTimer = null;
    }, 5000);
  }

  function clearUndo(): void {
    if (undoTimer) clearTimeout(undoTimer);
    undoTimer = null;
    undoOffer = null;
  }

  async function undoStep(): Promise<void> {
    const o = undoOffer;
    clearUndo();
    if (o) await toggleStep(o.id, o.index, false);
  }

  onDestroy(() => {
    if (undoTimer) clearTimeout(undoTimer);
  });

  async function loadSnapshot(): Promise<FeaturesLedger | null> {
    try {
      const res = await fetch('/features.json');
      if (!res.ok) return null;
      const snap = (await res.json()) as Partial<FeaturesLedger>;
      if (!Array.isArray(snap.features)) return null;
      return {
        updated: snap.updated ?? null,
        features: snap.features,
        generated_at: snap.generated_at
      };
    } catch {
      return null;
    }
  }

  // Fallback chain: live API, snapshot, demo.
  async function loadAll() {
    const b = await apiGetOr<FeaturesLedger>('/watch/features', demoFeatures);
    const snap = await loadSnapshot();
    if (b.live && Array.isArray(b.data?.features)) {
      ledger = b.data;
      live = true;
      staleNote = '';
    } else if (snap) {
      ledger = snap;
      live = false;
      staleNote = snap.generated_at
        ? `Snapshot from ${ageLabel(new Date(snap.generated_at))}.`
        : 'Baked snapshot. No live link.';
    } else {
      ledger = b.data;
      live = false;
      staleNote = b.stale ? `Last known from ${ageLabel(b.at)}.` : '';
    }
    updatedAt = new Date();
    loaded = true;
  }

  // Manual refresh keeps the old content up while it reloads.
  async function refresh() {
    if (refreshing) return;
    refreshing = true;
    try {
      await loadAll();
    } finally {
      refreshing = false;
    }
  }

  onMount(() => {
    void loadAll();
  });

  function nowStamp(): string {
    const d = new Date();
    const h = String(d.getHours()).padStart(2, '0');
    const min = String(d.getMinutes()).padStart(2, '0');
    return `${todayIso()} ${h}:${min}`;
  }

  async function confirmFeature(id: string, status: 'works' | 'broken'): Promise<void> {
    busyId = `${id}:${status}`;
    const today = todayIso();
    const stamp = nowStamp();
    try {
      const res = await apiPost<{ owner_confirmed_at?: string; feature?: FeatureRow }>(
        `/watch/features/${encodeURIComponent(id)}/confirm`,
        { status }
      );
      const dateVal = res.owner_confirmed_at ?? today;
      ledger = {
        ...ledger,
        features: ledger.features.map((f) =>
          f.id === id
            ? {
                ...f,
                owner_confirmed: status,
                owner_confirmed_at: dateVal,
                last_used: status === 'works' ? dateVal : f.last_used,
                completed_at:
                  status === 'works' && (!f.completed_at || !f.completed_at.trim())
                    ? stamp
                    : f.completed_at
              }
            : f
        )
      };
      saveStash('/watch/features', ledger);
      markNotice = {
        ...markNotice,
        [id]: status === 'works' ? `Confirmed works (${dateVal}).` : `Marked broken (${dateVal}).`
      };
    } catch (err) {
      const msg = (err as ApiError)?.message ?? 'Saved on this device only.';
      ledger = {
        ...ledger,
        features: ledger.features.map((f) =>
          f.id === id
            ? {
                ...f,
                owner_confirmed: status,
                owner_confirmed_at: today,
                last_used: status === 'works' ? today : f.last_used,
                completed_at:
                  status === 'works' && (!f.completed_at || !f.completed_at.trim())
                    ? stamp
                    : f.completed_at
              }
            : f
        )
      };
      saveStash('/watch/features', ledger);
      markNotice = { ...markNotice, [id]: msg };
    } finally {
      busyId = null;
    }
  }

  async function markFeature(id: string, kind: 'used' | 'tested_ok'): Promise<void> {
    busyId = `${id}:${kind}`;
    const today = todayIso();
    try {
      const res = await apiPost<{ date?: string }>(`/features/${encodeURIComponent(id)}/mark`, {
        kind
      });
      const dateVal = res.date ?? today;
      ledger = {
        ...ledger,
        features: ledger.features.map((f) =>
          f.id === id
            ? {
                ...f,
                last_used: kind === 'used' ? dateVal : f.last_used,
                tested: kind === 'tested_ok' ? dateVal : f.tested
              }
            : f
        )
      };
      saveStash('/watch/features', ledger);
      markNotice = {
        ...markNotice,
        [id]: kind === 'used' ? `Marked used (${dateVal}).` : `Marked tested OK (${dateVal}).`
      };
    } catch (err) {
      const msg = (err as ApiError)?.message ?? 'Saved on this device only.';
      ledger = {
        ...ledger,
        features: ledger.features.map((f) =>
          f.id === id
            ? {
                ...f,
                last_used: kind === 'used' ? today : f.last_used,
                tested: kind === 'tested_ok' ? today : f.tested
              }
            : f
        )
      };
      saveStash('/watch/features', ledger);
      markNotice = { ...markNotice, [id]: msg };
    } finally {
      busyId = null;
    }
  }

  async function toggleStep(
    id: string,
    stepIndex: number,
    forcedDone?: boolean
  ): Promise<{ done: boolean; error: string | null; message: string }> {
    busyId = `${id}:step:${stepIndex}`;
    const today = todayIso();
    const prev = ledger.features.find((f) => f.id === id)?.steps?.[stepIndex]?.done ?? false;
    const applyLocalToggle = (doneAtDate: string, forced?: boolean) => {
      ledger = {
        ...ledger,
        features: ledger.features.map((f) => {
          if (f.id !== id || !Array.isArray(f.steps)) return f;
          const nextSteps = f.steps.map((s, idx) => {
            if (idx !== stepIndex || s.owner !== 'you') return s;
            const nextDone = forced !== undefined ? forced : !s.done;
            return {
              ...s,
              done: nextDone,
              done_at: nextDone ? doneAtDate : ''
            };
          });
          return { ...f, steps: nextSteps };
        })
      };
      saveStash('/watch/features', ledger);
    };
    try {
      const body =
        forcedDone === undefined ? { index: stepIndex } : { index: stepIndex, done: forcedDone };
      const res = await apiPost<{ done?: boolean; done_at?: string }>(
        `/watch/features/${encodeURIComponent(id)}/step`,
        body
      );
      const nextDone = res.done ?? !prev;
      const dateVal = res.done_at ?? (nextDone ? today : '');
      applyLocalToggle(dateVal, nextDone);
      const okMsg = nextDone ? `Step marked done (${dateVal || today}).` : 'Step reopened.';
      markNotice = { ...markNotice, [id]: okMsg };
      return { done: nextDone, error: null, message: okMsg };
    } catch (err) {
      const apiErr = err as ApiError;
      const msg = apiErr?.message ?? 'Saved step on this device only.';
      const offline = apiErr?.status === undefined || apiErr.status === 0;
      const nextDone = forcedDone ?? !prev;
      applyLocalToggle(today, nextDone);
      markNotice = { ...markNotice, [id]: msg };
      return { done: nextDone, error: offline ? null : msg, message: msg };
    } finally {
      busyId = null;
    }
  }

  async function onStepToggle(
    id: string,
    index: number,
    want: boolean
  ): Promise<{ done: boolean; error: string | null }> {
    const r = await toggleStep(id, index, want);
    moveNotice = r.message;
    if (!r.error && r.done) {
      const text = ledger.features.find((f) => f.id === id)?.steps?.[index]?.text ?? 'step';
      offerUndo(id, index, text);
    } else {
      clearUndo();
    }
    return { done: r.done, error: r.error };
  }

  const yourMoves = () => yourMoveList(ledger.features);
  const attention = () => attentionList(ledger.features);
  const inProgress = () => inProgressList(ledger.features);
  const done = () => doneList(ledger.features);
  const areas = () => groupByArea(ledger.features);
</script>

{#snippet featureCard(f: FeatureRow, scope: string)}
  {@const tone = stageTone(f.stage)}
  {@const youTone = ownerConfirmTone(f)}
  {@const proof = proofText(f)}
  {@const quiet = isQuiet24h(f)}
  {@const stepProg = stepProgressLabel(f)}
  <article class="fcard">
    <div class="cardhead">
      <h3>{f.name}</h3>
      <div class="badges">
        <span class="chip" style="color: {tone}; border-color: {tone}">{claimedLabel(f)}</span>
        <span class="chip" style="color: {youTone}; border-color: {youTone}"
          >{ownerConfirmLabel(f)}</span
        >
        {#if stepProg}
          <span class="chip stepchip">{stepProg}</span>
        {/if}
        {#if quiet}
          <span class="chip quiet">Quiet 24 h</span>
        {/if}
      </div>
    </div>
    <p class="meta">Area: {f.area}{f.completed_at ? ` · Done ${f.completed_at}` : ''}</p>
    <dl class="lines">
      <div>
        <dt>Where it runs</dt>
        <dd>
          {#each splitWhereLinks(f.where) as part, idx (`${scope}-${f.id}-w-${idx}`)}
            {#if part.href}
              <a
                class="wlink"
                href={part.href}
                target={part.href.startsWith('http') ? '_blank' : undefined}
                rel={part.href.startsWith('http') ? 'noreferrer' : undefined}>{part.text}</a
              >
            {:else}
              {part.text}
            {/if}
          {/each}
        </dd>
      </div>
      <div>
        <dt>Proof</dt>
        <dd>
          {#if proof}
            {proof}
          {:else}
            <span class="noproof">no proof</span>
          {/if}
        </dd>
      </div>
      <div>
        <dt>Tested</dt>
        <dd>{f.tested}</dd>
      </div>
      <div>
        <dt>Deployed</dt>
        <dd>{f.deployed}</dd>
      </div>
      <div>
        <dt>Last used</dt>
        <dd>{f.last_used}</dd>
      </div>
      <div>
        <dt>Next</dt>
        <dd>{f.next}</dd>
      </div>
    </dl>
    {#if f.steps && f.steps.length > 0}
      <div class="stepsbox">
        <p class="stepshead">Steps ({stepProg})</p>
        <ol class="stepslist">
          {#each f.steps as step, sidx (`${scope}-${f.id}-s-${sidx}`)}
            <li class:done={step.done} class:you={step.owner === 'you'}>
              {#if step.owner === 'you'}
                <StepRow
                  item={{
                    featureId: f.id,
                    featureName: f.name,
                    area: f.area,
                    stepIndex: sidx,
                    text: step.text,
                    stepSummary: null,
                    where: step.where ?? '',
                    how: step.how ?? '',
                    cmd: step.cmd ?? ''
                  }}
                  done={step.done}
                  doneAt={step.done_at ?? ''}
                  busy={busyId === `${f.id}:step:${sidx}`}
                  onToggle={onStepToggle}
                />
              {:else}
                <div class="steprow">
                  <span class="sbox" aria-hidden="true">{step.done ? '✓' : '·'}</span>
                  <span class="stext">{step.text}</span>
                  <span class="sowner">agent{step.done && step.done_at ? ` · ${step.done_at}` : ''}</span>
                </div>
              {/if}
            </li>
          {/each}
        </ol>
      </div>
    {/if}
    <div class="actions">
      <button
        type="button"
        class="btn okbtn"
        disabled={busyId === `${f.id}:works`}
        onclick={() => confirmFeature(f.id, 'works')}
      >
        Works
      </button>
      <button
        type="button"
        class="btn badbtn"
        disabled={busyId === `${f.id}:broken`}
        onclick={() => confirmFeature(f.id, 'broken')}
      >
        Broken
      </button>
      <button
        type="button"
        class="btn solid"
        disabled={busyId === `${f.id}:used`}
        onclick={() => markFeature(f.id, 'used')}
      >
        I used it
      </button>
      <button
        type="button"
        class="btn"
        disabled={busyId === `${f.id}:tested_ok`}
        onclick={() => markFeature(f.id, 'tested_ok')}
      >
        Tested OK
      </button>
    </div>
    {#if markNotice[f.id]}
      <p class="notice" role="status">{markNotice[f.id]}</p>
    {/if}
    {#if undoOffer && undoOffer.id === f.id}
      <p class="notice" role="status">
        Marked done: {undoOffer.text}.
        <button type="button" class="undobtn" onclick={() => void undoStep()}>Undo</button>
      </p>
    {/if}
  </article>
{/snippet}

<main use:pullToRefresh={refresh}>
  <h1>Features</h1>
  <p class="sub">
    What is built, where it runs, what is deployed, tested, and confirmed by you.
    {ledger.updated ? ` Updated ${ledger.updated}.` : ''}
  </p>
  {#if !live && !staleNote}<p class="demo">Sample data.</p>{/if}
  {#if staleNote}<p class="demo">{staleNote}</p>{/if}
  <p class="sub">{refreshing ? 'Refreshing…' : updatedStamp(updatedAt)}</p>

  {#if loaded}
    <section class="tiles" aria-label="Feature counts">
      <div class="tile">
        <p class="k">Total</p>
        <p class="num">{ledger.features.length}</p>
      </div>
      <div class="tile">
        <p class="k">Needs attention</p>
        <p class="num warn">{attention().length}</p>
      </div>
      <div class="tile">
        <p class="k">In progress</p>
        <p class="num">{inProgress().length}</p>
      </div>
      <div class="tile">
        <p class="k">Done (you confirmed)</p>
        <p class="num ok">{done().length}</p>
      </div>
    </section>

    <section class="block" aria-label="Your move" id="your-move">
      <h2>Your move <span class="num count">{yourMoves().length}</span></h2>
      <p class="hint">Tap a step to see how.</p>
      {#if moveNotice}<p class="notice" role="status">{moveNotice}</p>{/if}
      {#if undoOffer}
        <p class="notice" role="status">
          Marked done: {undoOffer.text}.
          <button type="button" class="undobtn" onclick={() => void undoStep()}>Undo</button>
        </p>
      {/if}
      <p class="hint">Undone steps owned by you across all features, newest feature first. Tap a step to see how. The checkbox marks it done.</p>
      {#if yourMoves().length === 0}
        <p class="empty">No steps wait on you right now.</p>
      {:else}
        <ul class="yourmoves">
          {#each yourMoves() as item (`ym-${item.featureId}-${item.stepIndex}`)}
            <li>
              <StepRow
                {item}
                done={false}
                doneAt=""
                busy={busyId === `${item.featureId}:step:${item.stepIndex}`}
                onToggle={onStepToggle}
              />
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="block" aria-label="Needs attention">
      <h2>Needs attention <span class="num count">{attention().length}</span></h2>
      <p class="hint">Built, tested, or deployed and waiting on your confirmation, or marked broken.</p>
      {#if attention().length === 0}
        <p class="empty">Nothing waits for deploy or owner confirmation.</p>
      {:else}
        <div class="cards">
          {#each attention() as f (`att-${f.id}`)}
            {@render featureCard(f, 'att')}
          {/each}
        </div>
      {/if}
    </section>

    <section class="block" aria-label="In progress">
      <h2>In progress <span class="num count">{inProgress().length}</span></h2>
      <p class="hint">Not yet confirmed working by you.</p>
      {#if inProgress().length === 0}
        <p class="empty">No items in progress.</p>
      {:else}
        <div class="cards">
          {#each inProgress() as f (`prog-${f.id}`)}
            {@render featureCard(f, 'prog')}
          {/each}
        </div>
      {/if}
    </section>

    <section class="block" aria-label="Done">
      <h2>Done <span class="num count">{done().length}</span></h2>
      <p class="hint">Confirmed working by you, most recently completed on top.</p>
      {#if done().length === 0}
        <p class="empty">No owner-confirmed features yet. Tap Works on any feature you verified.</p>
      {:else}
        <div class="cards">
          {#each done() as f (`done-${f.id}`)}
            {@render featureCard(f, 'done')}
          {/each}
        </div>
      {/if}
    </section>

    <section class="block" aria-label="By area">
      <h2>All features by area</h2>
      {#each areas() as group (group.area)}
        <div class="areagroup">
          <h3 class="areahead">{group.area} <span class="num count">{group.items.length}</span></h3>
          <div class="cards">
            {#each group.items as f (`area-${group.area}-${f.id}`)}
              {@render featureCard(f, `area-${group.area}`)}
            {/each}
          </div>
        </div>
      {/each}
    </section>
  {:else}
    <section class="block skeleton" aria-busy="true">
      <div class="bar"></div>
      <div class="bar short"></div>
    </section>
  {/if}
</main>

<style>
  main {
    max-width: 68rem;
    margin: 0 auto;
    padding: 1.5rem 1rem 4.5rem;
  }
  @media (min-width: 48rem) {
    main {
      padding: 2rem 1.5rem 4rem;
    }
  }
  h1 {
    font-size: var(--text-display);
    margin: 0;
  }
  .sub {
    color: var(--paper-dim);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .demo {
    color: var(--accent);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.65rem;
    margin-top: 1rem;
  }
  @media (min-width: 48rem) {
    .tiles {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
  .tile {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 0.85rem 1rem;
  }
  .tile .k {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .tile .num {
    margin: 0;
    font-size: var(--text-xl);
    font-variant-numeric: var(--nums);
  }
  .tile .warn {
    color: var(--warn);
  }
  .tile .ok {
    color: var(--ok);
  }
  .block {
    margin-top: 1.5rem;
  }
  h2 {
    font-size: var(--text-lg);
    margin: 0;
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }
  .count {
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .hint {
    margin: 0.2rem 0 0.75rem;
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .empty {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 0.9rem 1rem;
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .areagroup {
    margin-top: 1rem;
  }
  .areahead {
    font-size: var(--text-md);
    margin: 0 0 0.6rem;
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
  }
  .cards {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.75rem;
  }
  @media (min-width: 56rem) {
    .cards {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .fcard {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1rem 1.1rem;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .cardhead {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.4rem;
  }
  .cardhead h3 {
    margin: 0;
    font-size: var(--text-md);
    line-height: 1.35;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: center;
  }
  .chip {
    flex: none;
    display: inline-block;
    padding: 0.12rem 0.55rem;
    border: 1px solid;
    border-radius: 999px;
    font-size: var(--text-xs);
    font-weight: 600;
    white-space: nowrap;
  }
  .chip.quiet {
    color: var(--warn);
    border-color: var(--warn);
  }
  .chip.stepchip {
    color: var(--accent);
    border-color: var(--accent);
  }
  .yourmoves {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.45rem;
  }
  @media (min-width: 56rem) {
    .yourmoves {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .stepsbox {
    margin-top: 0.65rem;
    padding-top: 0.55rem;
    border-top: 1px solid color-mix(in srgb, var(--paper-dim) 25%, transparent);
  }
  .stepshead {
    margin: 0 0 0.35rem;
    font-size: var(--text-xs);
    color: var(--paper-dim);
    font-weight: 600;
  }
  .stepslist {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .steprow {
    width: 100%;
    text-align: left;
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
    font: inherit;
    font-size: var(--text-xs);
    line-height: 1.35;
    padding: 0.15rem 0.25rem;
    color: var(--paper-dim);
  }
  .stepslist li.done .stext {
    text-decoration: line-through;
    opacity: 0.75;
  }
  .sbox {
    flex: none;
    color: var(--accent);
    font-weight: 600;
  }
  .stext {
    flex: 1;
    min-width: 0;
  }
  .sowner {
    flex: none;
    font-size: 0.7rem;
    color: var(--paper-dim);
  }
  .noproof {
    color: var(--warn);
    font-weight: 600;
  }
  .meta {
    margin: 0.35rem 0 0;
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .lines {
    margin: 0.65rem 0 0;
    display: grid;
    gap: 0.35rem;
    font-size: var(--text-sm);
  }
  .lines div {
    display: grid;
    grid-template-columns: 6.5rem minmax(0, 1fr);
    gap: 0.5rem;
    align-items: baseline;
  }
  .lines dt {
    color: var(--paper-dim);
    font-size: var(--text-xs);
  }
  .lines dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .wlink {
    color: var(--accent);
    text-decoration: underline;
    word-break: break-all;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }
  .btn {
    background: var(--ink);
    color: var(--paper);
    border: 1px solid var(--paper-dim);
    border-radius: 0.65rem;
    padding: 0.45rem 0.85rem;
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .btn.okbtn {
    background: color-mix(in srgb, var(--ok) 22%, var(--ink));
    color: var(--paper);
    border-color: var(--ok);
  }
  .btn.badbtn {
    background: color-mix(in srgb, var(--bad) 22%, var(--ink));
    color: var(--paper);
    border-color: var(--bad);
  }
  .btn.solid {
    background: var(--accent);
    color: var(--accent-ink);
    border-color: var(--accent);
  }
  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .notice {
    margin: 0.45rem 0 0;
    font-size: var(--text-xs);
    color: var(--ok);
  }
  .undobtn {
    background: none;
    border: none;
    padding: 0 0 0 0.5rem;
    color: var(--paper);
    font-size: var(--text-xs);
    text-decoration: underline;
    cursor: pointer;
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
