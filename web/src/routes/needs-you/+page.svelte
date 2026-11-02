<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { apiGet, ageLabel, loadStash, saveStash } from '$lib/api';
  import { DEMO_ONLY } from '$lib/config';
  import { pullToRefresh, updatedStamp } from '$lib/refresh';
  import {
    confirmScreen,
    isUsed,
    loadConfirms,
    loadUse,
    type ScreenConfirm,
    type ScreenUseMap
  } from '$lib/screenuse';
  import { splitWhereLinks, type FeatureRow, type FeaturesLedger } from '$lib/features';
  import { pickNext } from '$lib/nextstep';
  import { copyText as copyToClipboard } from '$lib/copy';
import {
    checkBridge,
    checkKey,
    checkUpdate,
    dataHealth,
    type HealthState
  } from '$lib/health';
  import {
    answerError,
    approveState,
    bridgeGet,
    bridgePost,
    codeExpired,
    codeMatches,
    previewLabel,
    QUICK_ACTIONS,
    BRIDGE_TEXT_MAX,
    type BridgeError,
    type BridgeOutput,
    type BridgePreview
  } from '$lib/bridge';
  import {
    demoNeedsYou,
    cardFor,
    countFeed,
    digestSegments,
    laneHref,
    pulseNode,
    sortThreads,
    stateAge,
    stripNodes,
    type NeedsYouFeed,
    type NeedThread,
    type NeedKind
  } from '$lib/needs-you';
  import YourMoveStrip from '$lib/YourMoveStrip.svelte';

  const KIND_PILL: Record<NeedKind, string> = {
    need: 'Needs answer',
    waiting: 'Waiting',
    building: 'Building',
    ready: 'Ready',
    paused: 'Paused',
    failed: 'Gone'
  };

  const KIND_TONE: Record<NeedKind, string> = {
    need: 'var(--hue-need)',
    waiting: 'var(--paper-dim)',
    building: 'var(--hue-business)',
    ready: 'var(--ok)',
    paused: 'var(--bad)',
    failed: 'var(--bad)'
  };



  const TAG_TONE: Record<string, string> = {
    NEED: 'var(--hue-need)',
    WORK: 'var(--hue-business)',
    DONE: 'var(--ok)',
    STALL: 'var(--warn)',
    PAUSE: 'var(--bad)',
    GONE: 'var(--bad)',
    IDLE: 'var(--paper-dim)'
  };

  // One plain line per card. Real states only.
  function cardLine(t: NeedThread): string {
    const k = cardFor(t);
    if (k === 'need') return t.question ?? 'Waits on the owner answer.';
    if (k === 'building') return 'Builds in the background.';
    if (k === 'ready') return 'Ready for review.';
    if (k === 'failed') return 'Session gone.';
    if (k === 'paused') return 'Pauses. No prompt shows.';
    return t.alerts[0] ?? 'Waits for the next task.';
  }

  let feed = $state<NeedsYouFeed>(demoNeedsYou);
  let live = $state(true);
  let staleNote = $state('');
  let loaded = $state(false);
  let updatedAt = $state<Date | null>(null);
  let refreshing = $state(false);
  let useMap = $state<ScreenUseMap>({});
  let confirms = $state<Record<string, ScreenConfirm>>({});
  let checkOpen = $state(false);
  let screenBusy = $state<string | null>(null);
  let screenResult = $state<Record<string, string>>({});
  let featureRows = $state<FeatureRow[]>([]);
  let nextCopyNote = $state<string | null>(null);
  const next = $derived(pickNext(feed, featureRows, useMap));

  const unopenedScreens = () => feed.unused.filter((u) => !isUsed(useMap, u.route));
  const openedScreens = () => feed.unused.filter((u) => isUsed(useMap, u.route));

  function toggleChecklist(): void {
    checkOpen = !checkOpen;
    if (checkOpen) useMap = loadUse();
  }

  async function confirmScreenRow(route: string, verdict: 'works' | 'broken'): Promise<void> {
    screenBusy = route;
    const r = await confirmScreen(route, verdict);
    if (r.ok) {
      confirms = { ...confirms, [route]: { confirmed: verdict, confirmed_at: r.confirmed_at } };
      screenResult = { ...screenResult, [route]: `Marked ${verdict}.` };
    } else {
      screenResult = { ...screenResult, [route]: `Failed: ${r.error}` };
    }
    screenBusy = null;
  }

  const counts = () => countFeed(feed);
  const firstNeed = () => feed.threads.find((t) => cardFor(t) === 'need') ?? null;

  type BridgeReady = { ok: true } | { ok: false; why: string };
  let bridge = $state<BridgeReady>({ ok: false, why: 'Checking the bridge…' });

  let health = $state<Record<'data' | 'bridge' | 'key' | 'update', HealthState>>({
    data: { ok: true, why: 'Checking…' },
    bridge: { ok: true, why: 'Checking…' },
    key: { ok: true, why: 'Checking…' },
    update: { ok: true, why: 'Checking…' }
  });

  const healthRows = () => [
    { name: 'Data link', s: health.data },
    { name: 'Bridge', s: health.bridge },
    { name: 'Key', s: health.key },
    { name: 'Update', s: health.update }
  ];

  type Panel = {
    pane: string;
    name: string;
    question: string | null;
    mode: 'answer' | 'approve' | 'screen';
    draft: string;
    draftError: string;
    busy: boolean;
    preview: BridgePreview | null;
    issuedAt: number;
    typed: string;
    sent: boolean;
    result: string;
    failed: boolean;
    lines: string[] | null;
    approvalWaiting: boolean;
    approvalPrompt: string | null;
    left: number;
  };
  let panel = $state<Panel | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  onDestroy(stopTimer);

  let approvals = $state<Record<string, boolean | null>>({});

  // One probe on open. It feeds the strip plus the card buttons.
  async function probeBridge() {
    const h = await checkBridge();
    health.bridge = h;
    bridge = h.ok ? { ok: true } : { ok: false, why: h.why };
  }

  // Approval flags per pane. Approve opens only on a waiting flag.
  async function probeApprovals(panes: string[]) {
    if (!bridge.ok) return;
    const jobs = [...new Set(panes.filter(Boolean))].slice(0, 15);
    await Promise.all(
      jobs.map(async (p) => {
        try {
          const out = await bridgeGet<BridgeOutput>(
            `/v1/panes/${encodeURIComponent(p)}/output?lines=50`
          );
          approvals = { ...approvals, [p]: out.approval_waiting };
        } catch {
          approvals = { ...approvals, [p]: null };
        }
      })
    );
  }

  function openPanel(t: NeedThread, mode: Panel['mode']) {
    stopTimer();
    panel = {
      pane: t.pane,
      name: t.name,
      question: t.question,
      mode,
      draft: '',
      draftError: '',
      busy: false,
      preview: null,
      issuedAt: 0,
      typed: '',
      sent: false,
      result: '',
      failed: false,
      lines: null,
      approvalWaiting: false,
      approvalPrompt: null,
      left: 0
    };
    if (mode === 'screen') void loadScreen();
  }

  function closePanel() {
    stopTimer();
    panel = null;
  }

  function startTimer() {
    stopTimer();
    timer = setInterval(() => {
      if (!panel?.preview) {
        stopTimer();
        return;
      }
      const left = Math.max(
        0,
        Math.round(panel.preview.expires_in - (Date.now() - panel.issuedAt) / 1000)
      );
      panel.left = left;
      if (left <= 0) stopTimer();
    }, 1000);
  }

  function stopTimer() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
  }

  // Screen is read-only. Failures fall back to the last known lines.
  async function loadScreen() {
    const p = panel;
    if (!p) return;
    p.busy = true;
    p.result = '';
    p.failed = false;
    try {
      const out = await bridgeGet<BridgeOutput>(
        `/v1/panes/${encodeURIComponent(p.pane)}/output?lines=50`
      );
      p.lines = out.lines;
      p.approvalWaiting = out.approval_waiting;
      p.approvalPrompt = out.approval_prompt;
      saveStash(`bridge:${p.pane}`, out.lines);
    } catch (e) {
      const err = e as BridgeError;
      p.failed = true;
      p.result = err.message;
      const old = loadStash<string[]>(`bridge:${p.pane}`);
      if (old?.data) {
        p.lines = old.data;
        p.result += ` Last known from ${ageLabel(old.at)}.`;
      }
    } finally {
      p.busy = false;
    }
  }

  // Preview first, always. The code in the answer comes from the bridge.
  // Quick taps pass fixed text. Drafts pass the typed paragraph.
  async function fetchPreview(quick?: string) {
    const p = panel;
    if (!p || p.mode === 'screen') return;
    const text = p.mode === 'answer' ? (quick ?? p.draft) : '';
    if (p.mode === 'answer') {
      const bad = answerError(text);
      if (bad) {
        p.draftError = bad;
        return;
      }
      p.draftError = '';
    }
    p.busy = true;
    p.result = '';
    p.failed = false;
    try {
      const pv = await bridgeGet<BridgePreview>(
        `/v1/panes/${encodeURIComponent(p.pane)}/message/preview?text=${encodeURIComponent(text)}`
      );
      p.preview = pv;
      p.issuedAt = Date.now();
      p.typed = '';
      p.sent = false;
      p.left = pv.expires_in;
      startTimer();
    } catch (e) {
      const err = e as BridgeError;
      p.failed = true;
      p.result = err.message;
    } finally {
      p.busy = false;
    }
  }

  // Confirm sends the previewed bytes plus Enter. Nothing else sends.
  async function confirmSend() {
    const p = panel;
    if (!p?.preview) return;
    if (codeExpired(p.issuedAt, p.preview.expires_in)) {
      p.failed = true;
      p.result = 'Code expired. Fetch a fresh preview.';
      return;
    }
    if (!codeMatches(p.typed, p.preview.code)) {
      p.failed = true;
      p.result = 'Code does not match. Type the shown code.';
      return;
    }
    p.busy = true;
    p.result = '';
    p.failed = false;
    try {
      await bridgePost<{ sent: boolean }>(`/v1/panes/${encodeURIComponent(p.pane)}/message`, {
        text: p.preview.text,
        code: p.preview.code
      });
      stopTimer();
      p.sent = true;
      p.result = p.mode === 'approve' ? 'Enter sent.' : 'Sent.';
    } catch (e) {
      const err = e as BridgeError;
      p.failed = true;
      p.result = err.message;
    } finally {
      p.busy = false;
    }
  }

  // The baked snapshot fills gaps when the API stays quiet.
  async function loadFeed(): Promise<NeedsYouFeed | null> {
    try {
      const res = await fetch('/needs-you.json');
      if (!res.ok) return null;
      const snap = (await res.json()) as Partial<NeedsYouFeed>;
      if (!Array.isArray(snap.threads) || !Array.isArray(snap.unused)) return null;
      return { events: [], log_at: null, ...snap } as NeedsYouFeed;
    } catch {
      return null;
    }
  }

  // Live API first. Then the baked snapshot. Demo fills last gaps.
  // The strip renders after every probe lands, so dots never flash.
  async function loadAll() {
    health.key = checkKey();
    const bridgeP = probeBridge();
    const updateP = checkUpdate().then((h) => (health.update = h));
    let liveData: NeedsYouFeed | null = null;
    let linkErr = '';
    // Demo mode never fetches, so it never logs console errors.
    if (!DEMO_ONLY) {
      try {
        liveData = (await apiGet<NeedsYouFeed>('/watch/needs-you')).data;
      } catch (e) {
        linkErr = (e as { message?: string }).message ?? 'No link.';
      }
    } else {
      linkErr = 'Demo mode. No live link.';
    }
    const snap = await loadFeed();
    if (liveData) {
      feed = liveData;
      live = true;
      staleNote = '';
    } else if (snap) {
      feed = snap;
      live = false;
      staleNote = snap.generated_at
        ? `Snapshot from ${ageLabel(new Date(snap.generated_at))}.`
        : 'Baked snapshot. No live link.';
    } else {
      const s = loadStash<NeedsYouFeed>('/watch/needs-you');
      if (s) {
        feed = s.data;
        staleNote = `Last known from ${ageLabel(s.at)}.`;
      } else {
        feed = demoNeedsYou;
        staleNote = '';
      }
      live = false;
    }
    health.data = dataHealth(liveData !== null, linkErr);
    await bridgeP;
    await updateP;
    void probeApprovals(feed.threads.map((t) => t.pane));
    useMap = loadUse();
    confirms = await loadConfirms();
    try {
      const feats = await apiGet<FeaturesLedger>('/watch/features');
      featureRows = feats?.data?.features ?? [];
    } catch {
      featureRows = [];
    }
    updatedAt = new Date();
    loaded = true;
  }

  async function copyNext(): Promise<void> {
    if (!next) return;
    const ok = await copyToClipboard(next.copyText);
    nextCopyNote = ok ? 'Copied.' : 'Copy failed. Long-press the text to copy it.';
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
</script>

<main use:pullToRefresh={refresh}>
  <h1>Needs you</h1>
  {#if !live && !staleNote}<p class="demo">Sample data.</p>{/if}
  {#if staleNote}<p class="demo">{staleNote}</p>{/if}
  <p class="age">{refreshing ? 'Refreshing…' : updatedStamp(updatedAt)}</p>
  {#if loaded}
    <section class="health" aria-label="Link health">
      {#each healthRows() as r (r.name)}
        <div class="hrow">
          <span class="hdot" class:ok={r.s.ok} aria-hidden="true"></span>
          <div class="hbody">
            <p class="hname">{r.name}</p>
            {#if !r.s.ok}<p class="hwhy">{r.s.why}</p>{/if}
          </div>
        </div>
      {/each}
      <button class="pbtn ghost" onclick={() => void refresh()}>Retry links</button>
    </section>
    <section class="tiles" aria-label="Counts">
      <a class="tile" href="/threads">
        <p class="k">Threads</p>
        <p class="num">{counts().threads}</p>
        <p class="meaning">sessions on the board.</p>
      </a>
      <a class="tile" href={laneHref('need')}>
        <p class="k">Need answer</p>
        <p class="num warn">{counts().need}</p>
        <p class="meaning">wait on your answer.</p>
      </a>
      <a class="tile" href={laneHref('waiting')}>
        <p class="k">Waiting</p>
        <p class="num wait">{counts().waiting}</p>
        <p class="meaning">wait calm for tasks.</p>
      </a>
      <a class="tile" href="#unused-check">
        <p class="k">Built</p>
        <p class="num ok">{counts().unused}</p>
        <p class="meaning">screens built, never opened.</p>
      </a>
    </section>
    <section class="digest" aria-label="Morning digest">
      <h2>Morning digest</h2>
      <p>
        {#each digestSegments(feed) as s, i}{#if i > 0}{'. '}{/if}<a
            class="dnum"
            href={s.href}>{s.n}</a
          >
          {s.text}{/each}.
      </p>
      <p class="age">
        {feed.log_at ? `Newest log line: ${feed.log_at}.` : 'No log yet.'}
      </p>
    </section>
    {#if next}
      <section class="nextcard" aria-label="Do this next">
        <h2>Do this next</h2>
        <p class="nextname">{next.name}</p>
        <p class="age">{next.context}</p>
        {#if next.where}
          <p class="nextline">
            Where: {#each splitWhereLinks(next.where) as part}{#if part.href}<a
                href={part.href}>{part.text}</a
              >{:else}{part.text}{/if}{/each}
          </p>
        {/if}
        {#if next.how}<p class="nextline">How: {next.how}</p>{/if}
        {#if next.cmd}<p class="nextline">Run: <code>{next.cmd}</code></p>{/if}
        <div class="nextbtns">
          <button type="button" class="pbtn ghost" onclick={() => void copyNext()}>Copy</button>
          <a
            class="btn solid"
            href={next.ctaHref}
            onclick={() => {
              if (next?.openChecklist) checkOpen = true;
            }}>{next.ctaLabel}</a
          >
        </div>
        {#if nextCopyNote}<p class="age">{nextCopyNote}</p>{/if}
      </section>
    {/if}
    <div class="grid">
      <div class="left">
        {#each sortThreads(feed.threads) as t (t.pane)}
          {@const k = cardFor(t)}
          {@const ap = approveState(bridge.ok, approvals[t.pane] ?? null)}
          <article class="tcard">
            <p class="tname">{t.name}</p>
            <p class="pill" style="color: {KIND_TONE[k]}; border-color: {KIND_TONE[k]}">
              {KIND_PILL[k]}
            </p>
            <p class="tline">{cardLine(t)}</p>
            <p class="age">{stateAge(t.changed_at)} in state.</p>
            <div class="brow">
              <button
                class="pbtn"
                disabled={!bridge.ok}
                onclick={() => openPanel(t, 'answer')}
              >
                Answer
              </button>
              <button
                class="pbtn"
                disabled={ap.disabled}
                onclick={() => openPanel(t, 'approve')}
              >
                Approve
              </button>
              <button
                class="pbtn"
                disabled={!bridge.ok}
                onclick={() => openPanel(t, 'screen')}
              >
                Screen
              </button>
            </div>
            {#if !bridge.ok && bridge.why}
              <p class="why">{bridge.why}</p>
            {:else if ap.reason}
              <p class="why">{ap.reason}</p>
            {/if}
            {#if panel && panel.pane === t.pane}
              <div class="panel">
                {#if panel.mode === 'screen'}
                  <h3>Screen: {panel.name}</h3>
                  {#if panel.approvalWaiting}
                    <p class="approve">
                      Approval waits.{#if panel.approvalPrompt} {panel.approvalPrompt}{/if}
                    </p>
                  {/if}
                  {#if panel.busy && !panel.lines}<p class="age">Reading…</p>{/if}
                  {#if panel.lines}<pre>{panel.lines.join('\n')}</pre>{/if}
                  {#if panel.result}
                    <p class:err={panel.failed} class:age={!panel.failed}>{panel.result}</p>
                  {/if}
                  <div class="prow">
                    <button class="pbtn" onclick={() => void loadScreen()} disabled={panel.busy}>
                      Refresh
                    </button>
                    <button class="pbtn ghost" onclick={closePanel}>Close</button>
                  </div>
                {:else if panel.sent}
                  <h3>{panel.mode === 'answer' ? 'Answer' : 'Approve'}: {panel.name}</h3>
                  <p class="sent">{panel.result}</p>
                  <div class="prow">
                    <button class="pbtn ghost" onclick={closePanel}>Close</button>
                  </div>
                {:else if !panel.preview}
                  <h3>{panel.mode === 'answer' ? `Answer ${panel.name}` : `Approve on ${panel.name}`}</h3>
                  {#if panel.mode === 'answer' && panel.question}
                    <p class="replyto">Reply to: {panel.question}</p>
                  {/if}
                  {#if panel.mode === 'answer'}
                    <div class="prow quick" aria-label="Quick sends">
                      {#each QUICK_ACTIONS as q (q.id)}
                        <button
                          class="pbtn"
                          onclick={() => void fetchPreview(q.text)}
                          disabled={panel.busy}
                        >
                          {q.label}
                        </button>
                      {/each}
                    </div>
                    <textarea
                      aria-label="Answer text"
                      placeholder="one line"
                      rows="1"
                      maxlength={BRIDGE_TEXT_MAX}
                      bind:value={panel.draft}
                      autocomplete="off"
                    ></textarea>
                    {#if panel.draftError}<p class="err">{panel.draftError}</p>{/if}
                  {:else}
                    <p class="age">Sends one Enter press. Needs a visible approval prompt.</p>
                  {/if}
                  {#if panel.result}<p class="err">{panel.result}</p>{/if}
                  <div class="prow">
                    <button class="pbtn" onclick={() => void fetchPreview()} disabled={panel.busy}>
                      {panel.mode === 'answer' ? 'Preview' : 'Preview Enter'}
                    </button>
                    <button class="pbtn ghost" onclick={closePanel}>Cancel</button>
                  </div>
                {:else}
                  <h3>Confirm on {panel.name}</h3>
                  <p class="pvtext">{previewLabel(panel.preview.text)}</p>
                  {#if panel.preview.approval_waiting}
                    <p class="approve">
                      Approval waits.{#if panel.preview.approval_prompt}
                        {panel.preview.approval_prompt}{/if}
                    </p>
                  {/if}
                  <p class="code" aria-label="Confirm code">{panel.preview.code}</p>
                  <p class="age">{panel.left} s left.</p>
                  {#if panel.left <= 0}<p class="err">Code expired. Fetch a fresh one.</p>{/if}
                  <input
                    aria-label="Type the shown code"
                    placeholder="Type the code"
                    bind:value={panel.typed}
                    autocomplete="off"
                  />
                  {#if panel.result}<p class:err={panel.failed}>{panel.result}</p>{/if}
                  <div class="prow">
                    <button
                      class="pbtn"
                      onclick={() => void confirmSend()}
                      disabled={panel.busy ||
                        panel.left <= 0 ||
                        !codeMatches(panel.typed, panel.preview.code)}
                    >
                      Confirm
                    </button>
                    <button class="pbtn ghost" onclick={() => void fetchPreview()} disabled={panel.busy}>
                      Fresh code
                    </button>
                    <button class="pbtn ghost" onclick={closePanel}>Cancel</button>
                  </div>
                {/if}
              </div>
            {/if}
          </article>
        {/each}
        <article class="tcard you">
          <p class="tname">You</p>
          <p class="pill" style="color: var(--paper-dim); border-color: var(--paper-dim)">Here</p>
          <p class="tline">Reads this board.</p>
        </article>
      </div>
      <div class="centre">
        <section class="strip" id="your-move" aria-label="Flow">
          {#each stripNodes(feed) as n, i (n.id)}
            {#if i > 0}
              <span class="seg" style="background: {n.tone}" aria-hidden="true"></span>
            {/if}
            <a
              class="fnode"
              class:pulse={pulseNode(feed) === n.id}
              href={n.href}
              aria-label="{n.label} {n.n}: {n.n} {n.why}"
              title="{n.n} {n.why}"
            >
              <span class="mark" style="color: {n.tone}">
                {#if n.id === 'building'}
                  <svg viewBox="0 0 40 44" aria-hidden="true">
                    <polygon
                      points="20,3 35,12 35,32 20,41 5,32 5,12"
                      fill="currentColor"
                      fill-opacity="0.15"
                      stroke="currentColor"
                      stroke-width="3"
                    />
                  </svg>
                {:else}
                  <span class="dot" aria-hidden="true"></span>
                {/if}
                <span class="mc num">{n.n}</span>
              </span>
              <span class="cap">{n.label}</span>
            </a>
          {/each}
        </section>
        <section class="log" aria-label="System log">
          <h2>System log</h2>
          {#if feed.events.length === 0}
            <p class="age">No changes on record.</p>
          {:else}
            <ol>
              {#each feed.events as e}
                <li>
                  <span class="age">{e.at}</span>
                  <span class="tag" style="color: {TAG_TONE[e.tag] ?? 'var(--paper-dim)'}">
                    {e.tag}
                  </span>
                  <span>{e.text}</span>
                </li>
              {/each}
            </ol>
          {/if}
        </section>
      </div>
      <aside class="right">
        <YourMoveStrip />
        <section class="nudge need" aria-label="Waiting for your answer">
          <h2>Waiting for your answer</h2>
          {#if firstNeed()}
            <p class="tname">{firstNeed()?.name}</p>
            <p>{firstNeed()?.question}</p>
            {#if counts().need > 1}<p class="age">{counts().need - 1} more wait.</p>{/if}
            <a class="btn solid" href="/threads">Respond</a>
          {:else}
            <p>None wait.</p>
          {/if}
        </section>
        <section class="checkcard" id="unused-check" aria-label="Screens to check">
          {#if unopenedScreens().length === 0}
            <p class="checkdone">Everything built is in use.</p>
          {:else}
            <div class="checkhead">
              <h2 class="checktitle">
                Check these {unopenedScreens().length} ({openedScreens().length} done)
              </h2>
              <button type="button" class="checktoggle" onclick={toggleChecklist}>
                {checkOpen ? 'Close checklist' : 'Open checklist'}
              </button>
            </div>
            {#if checkOpen}
              <ul class="checklist">
                {#each unopenedScreens() as u (u.route)}
                  <li class="checkrow">
                    <div class="checkline">
                      <span class="checkname">{u.name}</span>
                      <span class="checkused">
                        {isUsed(useMap, u.route) ? 'Opened.' : 'Not opened yet.'}
                      </span>
                      <a class="checktry" href={u.route}>Try it</a>
                      {#if confirms[u.route]}
                        <span class="checkverdict">
                          you: {confirms[u.route].confirmed} {confirms[u.route].confirmed_at}
                        </span>
                      {:else}
                        <button
                          type="button"
                          class="checkbtn"
                          disabled={screenBusy === u.route}
                          onclick={() => confirmScreenRow(u.route, 'works')}
                        >
                          Works
                        </button>
                        <button
                          type="button"
                          class="checkbtn bad"
                          disabled={screenBusy === u.route}
                          onclick={() => confirmScreenRow(u.route, 'broken')}
                        >
                          Broken
                        </button>
                      {/if}
                    </div>
                    {#if screenResult[u.route]}
                      <p class="checkresult">{screenResult[u.route]}</p>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          {/if}
        </section>
      </aside>
    </div>
  {:else}
    <section class="digest skeleton" aria-busy="true">
      <div class="bar"></div>
      <div class="bar short"></div>
    </section>
  {/if}
</main>

<style>
  main {
    max-width: 76rem;
    margin: 0 auto;
    padding: 2rem 1.25rem 4rem;
  }
  h1 {
    font-size: var(--text-display);
    margin: 0;
  }
  h2 {
    font-size: var(--text-md);
    margin: 0 0 0.5rem;
  }
  .demo {
    color: var(--accent);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .age {
    color: var(--paper-dim);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
    margin-top: 1rem;
  }
  @media (min-width: 56rem) {
    .tiles {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
  .tile {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 0.9rem 1rem;
    display: block;
    text-decoration: none;
    color: inherit;
  }
  .tile .meaning {
    margin: 0.15rem 0 0;
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .dnum {
    color: var(--accent);
    font-variant-numeric: var(--nums);
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
    color: var(--hue-need);
  }
  .tile .wait {
    color: var(--paper-dim);
  }
  .tile .ok {
    color: var(--ok);
  }
  .digest {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1rem 1.25rem;
    margin-top: 1rem;
  }
  .digest p {
    margin: 0.25rem 0 0;
  }
  .nextcard {
    border-radius: 1rem;
    padding: 1rem 1.25rem;
    margin-top: 1rem;
    background: color-mix(in srgb, var(--hue-need) 14%, var(--ink-raise));
  }
  .nextcard h2 {
    font-size: var(--text-sm);
    margin: 0;
  }
  .nextname {
    font-size: var(--text-base);
    font-weight: 600;
    margin: 0.25rem 0 0;
  }
  .nextline {
    margin: 0.25rem 0 0;
    font-size: var(--text-sm);
  }
  .nextline code {
    font-size: var(--text-xs);
  }
  .nextbtns {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 0.75rem;
    margin-top: 0.75rem;
    align-items: center;
  }
  .grid {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-top: 1rem;
  }
  .left,
  .centre,
  .right {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
  }
  @media (min-width: 56rem) {
    .grid {
      display: grid;
      grid-template-columns: minmax(0, 5fr) minmax(0, 7fr) minmax(0, 5fr);
      align-items: start;
    }
  }
  /* Phone order: strip, nudges, thread cards, log. */
  @media (max-width: 55.99rem) {
    .centre {
      display: contents;
    }
    .strip {
      order: -3;
    }
    .right {
      order: -2;
    }
    .left {
      order: -1;
    }
  }
  .tcard {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 0.9rem 1rem;
  }
  .tname {
    margin: 0;
    font-size: var(--text-md);
  }
  .pill {
    display: inline-block;
    margin: 0.35rem 0 0;
    padding: 0.1rem 0.6rem;
    border: 1px solid;
    border-radius: 999px;
    font-size: var(--text-xs);
  }
  .tline {
    margin: 0.4rem 0 0;
    font-size: var(--text-sm);
  }
  .btn {
    display: inline-block;
    margin-top: 0.6rem;
    background: var(--ink);
    color: var(--paper);
    border-radius: 0.75rem;
    padding: 0.45rem 0.9rem;
    font-size: var(--text-sm);
    text-decoration: none;
  }
  .btn.solid {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .health {
    display: grid;
    gap: 0.6rem;
    margin-top: 1rem;
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 0.9rem 1rem;
  }
  @media (min-width: 56rem) {
    .health {
      grid-template-columns: repeat(4, minmax(0, 1fr)) auto;
      align-items: start;
    }
  }
  .hrow {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
  }
  .hdot {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--bad);
    flex: none;
    transform: translateY(-1px);
  }
  .hdot.ok {
    background: var(--ok);
  }
  .hname {
    margin: 0;
    font-size: var(--text-sm);
  }
  .hwhy {
    margin: 0.15rem 0 0;
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .brow {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.6rem;
  }
  .why {
    margin: 0.4rem 0 0;
    font-size: var(--text-xs);
    color: var(--paper-dim);
  }
  .replyto {
    margin: 0.4rem 0 0;
    font-size: var(--text-sm);
  }
  .pbtn {
    background: var(--ink);
    color: var(--paper);
    border: none;
    border-radius: 0.75rem;
    padding: 0.45rem 0.9rem;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .pbtn.ghost {
    background: none;
    color: var(--paper-dim);
    text-decoration: underline;
  }
  .pbtn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .panel {
    border-top: 1px solid var(--ink);
    margin-top: 0.6rem;
    padding-top: 0.6rem;
  }
  .panel h3 {
    margin: 0 0 0.4rem;
    font-size: var(--text-sm);
  }
  .panel input,
  .panel textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--ink);
    color: var(--paper);
    border: none;
    border-radius: 0.75rem;
    padding: 0.5rem 0.75rem;
    font-size: var(--text-sm);
    font-family: inherit;
    resize: vertical;
  }
  .panel pre {
    background: var(--ink);
    border-radius: 0.75rem;
    padding: 0.6rem 0.75rem;
    font-size: var(--text-xs);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 12rem;
    overflow-y: auto;
    margin: 0.4rem 0 0;
  }
  .prow {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.6rem;
    flex-wrap: wrap;
  }
  .pvtext {
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
    word-break: break-word;
  }
  .code {
    font-size: var(--text-xl);
    font-variant-numeric: var(--nums);
    letter-spacing: 0.15em;
    margin: 0.4rem 0 0;
  }
  .approve {
    color: var(--hue-need);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .sent {
    color: var(--ok);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .err {
    color: var(--bad);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .strip {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.35rem;
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1rem 1.25rem;
  }
  @media (min-width: 56rem) {
    .strip {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
  }
  .fnode {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.75rem;
    text-decoration: none;
    color: inherit;
  }
  @media (min-width: 56rem) {
    .fnode {
      flex-direction: column;
      gap: 0.35rem;
    }
  }
  .mark {
    position: relative;
    flex: none;
    width: 2.4rem;
    height: 2.4rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .mark .dot {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: currentColor;
  }
  .mark svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  /* Fixed dark ink reads on yellow, green, and red in both themes. */
  .mc {
    position: relative;
    color: #241b12;
    font-weight: 600;
    font-size: var(--text-md);
    font-variant-numeric: var(--nums);
  }
  .cap {
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  @media (min-width: 56rem) {
    .cap {
      min-height: 1.5rem;
    }
  }
  .fnode.pulse .mark::after {
    content: '';
    position: absolute;
    inset: -6px;
    border: 2px solid currentColor;
    border-radius: 50%;
    animation: ring 1.6s var(--ease-out) infinite;
  }
  @keyframes ring {
    from {
      transform: scale(0.85);
      opacity: 0.9;
    }
    to {
      transform: scale(1.35);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fnode.pulse .mark::after {
      animation: none;
      opacity: 0.9;
    }
  }
  .seg {
    width: 3px;
    height: 1rem;
    margin-left: 1.1rem;
    border-radius: 2px;
    flex: none;
  }
  @media (min-width: 56rem) {
    .seg {
      width: auto;
      height: 3px;
      flex: 1 1 0;
      margin-left: 0;
      margin-bottom: 1.5rem;
      align-self: center;
    }
  }
  .log {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1rem 1.25rem;
  }
  .log ol {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: var(--text-sm);
  }
  .log li {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: baseline;
    padding: 0.3rem 0;
    border-top: 1px solid var(--ink);
  }
  .log li:first-child {
    border-top: none;
  }
  .log .age {
    font-variant-numeric: var(--nums);
  }
  .tag {
    font-size: var(--text-xs);
  }
  @media (max-width: 55.99rem) {
    .log li:nth-child(n + 5) {
      display: none;
    }
  }
  .nudge {
    border-radius: 1rem;
    padding: 1rem 1.25rem;
  }
  .nudge.need {
    background: color-mix(in srgb, var(--hue-need) 14%, var(--ink-raise));
  }
  .nudge p {
    margin: 0.25rem 0 0;
    font-size: var(--text-sm);
  }
  .checkcard {
    border-radius: 1rem;
    padding: 1rem 1.25rem;
    background: color-mix(in srgb, var(--ok) 12%, var(--ink-raise));
    scroll-margin-top: 1rem;
  }
  .checkdone {
    margin: 0;
    font-size: var(--text-sm);
  }
  .checkhead {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem 1rem;
  }
  .checktitle {
    font-size: var(--text-base);
    margin: 0;
  }
  .checktoggle {
    font-size: var(--text-sm);
    text-decoration: underline;
    background: none;
    border: none;
    padding: 0.25rem 0;
    cursor: pointer;
    color: inherit;
  }
  .checklist {
    list-style: none;
    margin: 0.75rem 0 0;
    padding: 0;
  }
  .checkrow {
    padding: 0.5rem 0;
    border-top: 1px solid color-mix(in srgb, var(--ink) 12%, transparent);
  }
  .checkline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem 0.75rem;
    font-size: var(--text-sm);
  }
  .checkname {
    font-weight: 600;
  }
  .checkused {
    font-size: var(--text-xs);
  }
  .checktry {
    font-weight: 600;
  }
  .checkverdict {
    font-size: var(--text-xs);
  }
  .checkbtn {
    font-size: var(--text-xs);
    padding: 0.35rem 0.75rem;
    border-radius: 0.5rem;
    border: 1px solid color-mix(in srgb, var(--ink) 25%, transparent);
    background: var(--paper);
    cursor: pointer;
    color: inherit;
  }
  .checkbtn.bad {
    color: var(--bad);
    border-color: color-mix(in srgb, var(--bad) 45%, transparent);
  }
  .checkbtn:disabled {
    opacity: 0.5;
    cursor: wait;
  }
  .checkresult {
    margin: 0.25rem 0 0;
    font-size: var(--text-xs);
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
