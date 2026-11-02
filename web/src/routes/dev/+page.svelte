<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr } from '$lib/api';
  import { bridgeBase, bridgeGet, type BridgeOutput } from '$lib/bridge';
  import { copyText } from '$lib/copy';
  import {
    activityLine,
    arcLabel,
    chipCounts,
    EMPTY_DEV_FEED,
    hasFlow,
    hexTop,
    laneColor,
    LANE_TONE,
    lastCommitFor,
    matchLane,
    minsSince,
    nodeAge,
    nodeState,
    ribbonPath,
    ribbonWidth,
    selectSlots,
    stampHM,
    staleWorking,
    timelineRows,
    type DevFeed,
    type DevState,
    type LaneMeta
  } from '$lib/dev';
  import { demoNeedsYou, type NeedsYouFeed, type NeedThread } from '$lib/needs-you';
  import {
    answerLine,
    bridgeOff,
    commitLine,
    laneLine,
    progressUnknown,
    resultLine,
    spendLine,
    staleLine
  } from '$lib/plain';

  type BridgePane = { id: string; label: string; tab: string; kind: string; status: string };

  type DevNode = {
    key: string;
    name: string;
    pane: string;
    thread: NeedThread | null;
    lane: LaneMeta | null;
  };

  let feed = $state<DevFeed>(EMPTY_DEV_FEED);
  let threads = $state<NeedThread[]>([]);
  let live = $state(false);
  let loaded = $state(false);
  let loadedAt = $state(new Date());
  let approvals = $state<Record<string, boolean>>({});
  let screens = $state<Record<string, string[]>>({});
  let bridgeWhy = $state('');
  let bridgeDown = $state(false);
  let copyNote = $state('');
  let sheetKey = $state<string | null>(null);
  let sheetLines = $state<string[]>([]);
  let sheetErr = $state('');

  const nodes = $derived(buildNodes(threads, feed.lanes));
  const chips = $derived(
    chipCounts(threads, Object.values(approvals).filter(Boolean).length)
  );
  const rows = $derived(timelineRows(feed.commits, feed.tests));
  const calm = $derived(threads.length === 0 && feed.lanes.length === 0);
  const sheetNode = $derived(nodes.find((n) => n.key === sheetKey) ?? null);

  function buildNodes(ts: NeedThread[], lanes: LaneMeta[]): DevNode[] {
    const out: DevNode[] = ts.map((t, i) => ({
      key: `t:${t.pane || i}`,
      name: t.name,
      pane: t.pane,
      thread: t,
      lane: matchLane(t, lanes)
    }));
    const used = new Set(out.map((n) => n.lane?.lane ?? ''));
    for (const l of lanes) {
      if (used.has(l.lane)) continue;
      out.push({ key: `l:${l.lane}`, name: l.lane, pane: '', thread: null, lane: l });
    }
    return out;
  }

  function stateOf(n: DevNode): { label: string; reason: string; key: DevState } {
    if (!n.thread) return { label: 'Idle', reason: 'No session runs.', key: 'idle' };
    const s = nodeState(n.thread);
    return { label: s.label, reason: s.reason, key: s.key };
  }

  function colorOf(n: DevNode): { state: DevState; idleMins: number | null; stale: boolean; approval: boolean } {
    const key = stateOf(n).key;
    return {
      state: key,
      idleMins: n.thread ? minsSince(n.thread.changed_at) : null,
      stale: n.thread ? staleWorking(n.thread) : false,
      approval: n.pane ? approvals[n.pane] === true : false
    };
  }

  function toneOf(n: DevNode): string {
    return LANE_TONE[laneColor(colorOf(n))];
  }

  function flowOf(n: DevNode): boolean {
    return hasFlow(colorOf(n));
  }

  function stateWord(n: DevNode): string {
    const k = stateOf(n).key;
    if (k === 'waiting') return 'waiting for you';
    return k;
  }

  function screenLast(pane: string): string {
    const lines = screens[pane] ?? [];
    for (let i = lines.length - 1; i >= 0; i--) {
      const t = lines[i].trim();
      if (t) return t.slice(0, 140);
    }
    return '';
  }

  function describe(n: DevNode): string {
    const job = n.lane?.note || (n.thread ? activityLine(n.thread, n.lane, screenLast(n.pane)) : '');
    const rel = n.thread ? arcLabel(n.thread, feed.lanes) : '';
    let line = laneLine(n.name, job === 'Unknown.' ? '' : job, stateWord(n), rel);
    if (n.thread?.question) line += ` ${answerLine(1)}`;
    return line;
  }

  function openWaits(n: DevNode): number {
    let c = n.thread?.question ? 1 : 0;
    if (n.pane && approvals[n.pane]) c += 1;
    return c;
  }

  function decisions(): { text: string; href: string; label: string }[] {
    const out: { text: string; href: string; label: string }[] = [];
    for (const t of threads) {
      if (t.question) out.push({ text: t.question, href: '/threads?lane=need', label: 'Answer' });
    }
    for (const n of nodes) {
      if (n.pane && approvals[n.pane]) {
        out.push({ text: `Approve on ${n.name}.`, href: '/threads', label: 'Approve' });
      }
    }
    return out;
  }

  function fmtTime(at: string): string {
    if (!at) return 'time unknown';
    const d = new Date(at);
    if (Number.isNaN(d.getTime())) return at;
    return stampHM(d);
  }

  async function probeBridge(panes: string[]): Promise<void> {
    bridgeDown = false;
    bridgeWhy = '';
    if (!bridgeBase()) {
      bridgeWhy = bridgeOff();
      return;
    }
    try {
      const list = await bridgeGet<{ panes: BridgePane[] }>('/v1/panes');
      const ids = new Set(list.panes.map((p) => p.id));
      const jobs = panes.filter((p) => p && ids.has(p)).slice(0, 15);
      await Promise.all(
        jobs.map(async (p) => {
          try {
            const out = await bridgeGet<BridgeOutput>(
              `/v1/panes/${encodeURIComponent(p)}/output?lines=20`
            );
            approvals = { ...approvals, [p]: out.approval_waiting };
            screens = { ...screens, [p]: out.lines ?? [] };
          } catch {
            /* one dark pane never blocks the board */
          }
        })
      );
    } catch (e) {
      bridgeDown = true;
      bridgeWhy = (e as { message?: string })?.message ?? 'Bridge is down. Retry soon.';
    }
  }

  async function loadAll(): Promise<void> {
    try {
      const [n, d] = await Promise.all([
        apiGetOr<NeedsYouFeed>('/watch/needs-you', demoNeedsYou),
        apiGetOr<DevFeed>('/watch/dev/feed', EMPTY_DEV_FEED)
      ]);
      threads = n.data?.threads ?? [];
      feed = d.data ?? EMPTY_DEV_FEED;
      live = n.live || d.live;
    } catch {
      live = false;
    } finally {
      loadedAt = new Date();
      loaded = true;
    }
    await probeBridge(threads.map((t) => t.pane));
  }

  async function openSheet(n: DevNode): Promise<void> {
    sheetKey = n.key;
    sheetErr = '';
    sheetLines = n.pane ? (screens[n.pane] ?? []) : [];
    if (n.pane && sheetLines.length === 0 && bridgeBase()) {
      try {
        const out = await bridgeGet<BridgeOutput>(
          `/v1/panes/${encodeURIComponent(n.pane)}/output?lines=20`
        );
        sheetLines = out.lines ?? [];
        screens = { ...screens, [n.pane]: sheetLines };
      } catch (e) {
        sheetErr = `Failed: ${(e as { message?: string })?.message ?? 'Screen is dark. Retry.'}`;
      }
    } else if (n.pane && !bridgeBase()) {
      sheetErr = bridgeOff();
    }
  }

  function closeSheet(): void {
    sheetKey = null;
    sheetLines = [];
    sheetErr = '';
  }

  async function copyBridge(): Promise<void> {
    copyNote = (await copyText('lo-bridge')) ? 'Copied. Run it where the bridge lives.' : 'Copy failed.';
  }

  function pauseMotion(): void {
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
      for (const id of ['ribbons', 'arcs']) {
        const s = document.getElementById(id) as unknown as {
          pauseAnimations?: () => void;
        } | null;
        s?.pauseAnimations?.();
      }
    }
  }

  const pick = $derived(
    selectSlots(nodes.map((n) => ({ id: n.key, state: stateOf(n).key })))
  );
  const slotNodes = $derived(
    pick.shown
      .map((k) => nodes.find((n) => n.key === k))
      .filter((n) => n !== undefined)
  );
  const slotRows = $derived(Math.max(1, Math.ceil(slotNodes.length / 2)));
  const slotTop = $derived(hexTop(slotRows));
  const slotDy = $derived(slotTop - 392);
  const liveNodes = $derived(nodes.filter((n) => stateOf(n).key !== 'done'));
  const doneNodes = $derived(nodes.filter((n) => stateOf(n).key === 'done'));
  let showDone = $state(false);

  function ribbonSlots(): { n: DevNode; x: number; y: number }[] {
    return slotNodes.map((n, i) => ({
      n,
      x: i % 2 === 0 ? 8 : 200,
      y: 8 + Math.floor(i / 2) * 88
    }));
  }

  function flowSlots(): number[] {
    const out: number[] = [];
    ribbonSlots().forEach((s, i) => {
      if (out.length < 3 && flowOf(s.n)) out.push(i);
    });
    return out;
  }

  onMount(() => {
    void loadAll().then(pauseMotion);
  });
</script>

<main>
  <p class="secttl">DEV</p>
  {#if !live && loaded}<p class="demo">Sample data.</p>{/if}
  <p class="age">Last updated {stampHM(loadedAt)}.</p>

  {#if loaded && calm}
    <div class="calmbox">
      <h2>No jobs run now.</h2>
      <p>Hearth shows the flow when a job starts. Nothing is hidden.</p>
      <button class="btn primary" type="button" onclick={() => void loadAll()}>Retry</button>
    </div>
  {:else}
    <div class="chips" aria-label="Status">
      <span class="chip"><span class="dot" style="background:var(--bad)" aria-hidden="true"></span><strong>{chips.stalled}</strong> <small>stalled</small></span>
      <span class="chip"><span class="dot" style="background:var(--wait)" aria-hidden="true"></span><strong>{chips.need}</strong> <small>needs answer</small></span>
      <span class="chip"><span class="dot live" style="background:var(--ok)" aria-hidden="true"></span><strong>{chips.building}</strong> <small>building</small></span>
    </div>

    {#if bridgeDown}
      <div class="warn" role="alert">
        <h3>Bridge is down</h3>
        <p>{bridgeWhy} Hearth reads saved screens only.</p>
        <div class="cmdline"><code>lo-bridge</code><button class="btn ghost" type="button" onclick={() => void copyBridge()}>Copy</button></div>
        {#if copyNote}<p class="picked">{copyNote}</p>{/if}
        <button class="btn primary" type="button" onclick={() => void probeBridge(threads.map((t) => t.pane))}>Try again</button>
      </div>
    {/if}

    <section class="youcard" aria-label="Your turn">
      <h2>YOU</h2>
      {#if decisions().length === 0}
        <p>Nothing waits on you.</p>
      {:else}
        <ul>
          {#each decisions().slice(0, 4) as d}
            <li><span>{d.text}</span><a class="btn-big" href={d.href}>{d.label}</a></li>
          {/each}
        </ul>
        {#if decisions().length > 4}<p class="age">{decisions().length - 4} more wait in Talk.</p>{/if}
      {/if}
    </section>

    {#if feed.lanes_note && nodes.length === 0}
      <p class="age">{feed.lanes_note}</p>
    {/if}

    <svg id="ribbons" class="ribbons" viewBox={`0 0 360 ${slotTop + 168}`} role="img" aria-label="Agents flow to you. Ribbon width means work waiting.">
      <defs>
        {#each ribbonSlots() as s, i}
          <path id={`rb${i}`} d={ribbonPath(i, slotRows, slotTop)} fill="none" />
        {/each}
      </defs>
      {#each ribbonSlots() as s, i}
        <use
          href={`#rb${i}`}
          stroke={toneOf(s.n)}
          stroke-width={ribbonWidth(openWaits(s.n))}
          opacity="0.75"
          stroke-linecap="round"
        />
      {/each}
      {#each flowSlots() as i}
        <circle r="6" fill="#ffd9a3">
          <animateMotion dur={`${3.5 + (i % 3)}s`} repeatCount="indefinite"><mpath href={`#rb${i}`} /></animateMotion>
        </circle>
      {/each}
      {#each ribbonSlots() as s, i}
        {@const st = stateOf(s.n)}
        {@const pct = s.n.lane?.pct}
        <g
          role="button"
          tabindex="0"
          aria-label={`${s.n.name}, ${st.label}`}
          onclick={() => void openSheet(s.n)}
          onkeydown={(e) => {
            if (e.key === 'Enter') void openSheet(s.n);
          }}
        >
          <rect x={s.x} y={s.y} width="152" height="64" rx="12" fill="var(--ink-raise)" stroke="color-mix(in srgb, var(--accent) 30%, transparent)" />
          <circle cx={s.x + 26} cy={s.y + 32} r="16" fill="none" stroke="color-mix(in srgb, var(--paper) 14%, transparent)" stroke-width="7" />
          {#if pct !== null && pct !== undefined}
            <circle cx={s.x + 26} cy={s.y + 32} r="16" fill="none" stroke="var(--accent)" stroke-width="7" stroke-linecap="round" stroke-dasharray="100.5" stroke-dashoffset={100.5 * (1 - Math.min(100, Math.max(0, pct)) / 100)} transform={`rotate(-90 ${s.x + 26} ${s.y + 32})`} />
          {/if}
          <text x={s.x + 50} y={s.y + 29} fill="var(--paper)" font-size="14" font-weight="700">{s.n.name.slice(0, 10).toUpperCase()}</text>
          <text x={s.x + 50} y={s.y + 47} fill="var(--paper-dim)" font-size="11">
            {pct !== null && pct !== undefined ? `${pct}% · ` : ''}{st.label.toLowerCase()}
          </text>
          <circle cx={s.x + 142} cy={s.y + 16} r="6" fill={toneOf(s.n)} />
        </g>
      {/each}
      <polygon
        points={`180,${395 + slotDy} 231,${424 + slotDy} 231,${483 + slotDy} 180,${512 + slotDy} 129,${483 + slotDy} 129,${424 + slotDy}`}
        fill="#2a1c08"
        stroke="var(--accent)"
        stroke-width="3"
      />
      <circle cx="180" cy={445 + slotDy} r="20" fill="none" stroke="var(--accent)" stroke-width="4" />
      <text x="180" y={482 + slotDy} text-anchor="middle" fill="var(--paper)" font-size="19" font-weight="800">YOU</text>
      <text x="180" y={535 + slotDy} text-anchor="middle" fill="var(--paper-dim)" font-size="13">
        {chips.need} wait on you
      </text>
    </svg>
    {#if pick.more > 0}
      <p class="age">{pick.more} more lanes in the list below.</p>
    {/if}
    {#if doneNodes.length > 0}
      <button class="btn ghost donechip" type="button" onclick={() => (showDone = !showDone)}>
        Done: {doneNodes.length}
      </button>
      {#if showDone}
        <ul class="donelist">
          {#each doneNodes as n}
            <li>
              <button type="button" class="donerow" onclick={() => void openSheet(n)}>
                <span>{n.name}</span>
                <span class="age">{n.thread?.changed_at ? `done ${nodeAge(n.thread)} ago` : 'done.'}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    <div class="flowlanes">
      {#each liveNodes as n}
        {@const st = stateOf(n)}
        {@const mins = n.thread ? minsSince(n.thread.changed_at) : null}
        {@const stale = n.thread ? staleWorking(n.thread) : false}
        {@const commit = n.lane ? lastCommitFor(n.lane.lane, feed.commits) : null}
        <article class="node">
          <button type="button" class="nhead" onclick={() => void openSheet(n)} aria-label={`Open ${n.name}`}>
            <strong>{n.name.toUpperCase()}</strong>
            <span class="st">{st.label} <span class="dot" class:live={flowOf(n)} style="background:{toneOf(n)}" aria-hidden="true"></span></span>
          </button>
          <p class="nline">{describe(n)}</p>
          {#if n.thread}
            <p class="ns">
              {activityLine(n.thread, n.lane, screenLast(n.pane))}
              {#if mins !== null}for {mins} min.{/if}
            </p>
          {/if}
          {#if n.lane?.pct !== null && n.lane?.pct !== undefined}
            <div class="qbar" role="img" aria-label={`${n.lane.pct} percent`}>
              <i style="width:{Math.min(100, Math.max(0, n.lane.pct))}%;background:{toneOf(n)};"></i>
              <span>{n.lane.pct}%</span>
            </div>
          {:else}
            <p class="age">{progressUnknown()}</p>
          {/if}
          {#if n.lane && n.lane.rows !== null}
            <p class="nmeta">{resultLine(n.lane.rows ?? 0, n.lane.errors ?? 0)}</p>
          {/if}
          {#if commit}
            <p class="nmeta">{commitLine(commit.sha.slice(0, 7), nodeAge({ ...n.thread, changed_at: commit.at } as NeedThread))}</p>
          {/if}
          {#if n.lane?.spend}
            <p class="nmeta">{spendLine(n.lane.spend)}</p>
          {/if}
          {#if stale && mins !== null}
            <p class="warnline">{staleLine(mins)}</p>
          {/if}
        </article>
      {/each}
    </div>

    <section class="legend" aria-label="Legend">
      <h2>Legend</h2>
      <ul>
        <li><span class="dot" style="background:var(--ok)" aria-hidden="true"></span> Green builds or done.</li>
        <li><span class="dot" style="background:var(--wait)" aria-hidden="true"></span> Yellow waits for you.</li>
        <li><span class="dot" style="background:var(--bad)" aria-hidden="true"></span> Red is blocked or stuck.</li>
        <li><span class="dot" style="background:var(--paper-dim)" aria-hidden="true"></span> Grey rests idle.</li>
        <li><span class="ribbon" aria-hidden="true"></span> Wide ribbon means more waits on you.</li>
        <li><span class="mdot" aria-hidden="true"></span> Moving dot means work flows.</li>
      </ul>
    </section>

    <section class="timeline" aria-label="Commits and tests">
      <h2>Commits and tests</h2>
      {#if rows.length === 0}
        <p class="age">{feed.commits_note || feed.tests_note || 'Nothing on record.'}</p>
      {:else}
        <ul>
          {#each rows.slice(0, 12) as r}
            <li>
              <span class="dot" style="background:{r.tone}" aria-hidden="true"></span>
              <span class="t">{r.title}<small>{fmtTime(r.sub)}</small></span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="health" aria-label="Link health">
      <h2>Links</h2>
      <ul>
        <li>
          <span class="dot" style="background:{live ? 'var(--ok)' : 'var(--bad)'}" aria-hidden="true"></span>
          <b>Data link</b> {live ? 'live.' : 'down. Hearth keeps last data.'}
        </li>
        <li>
          <span class="dot" style="background:{bridgeDown || bridgeWhy ? 'var(--bad)' : 'var(--ok)'}" aria-hidden="true"></span>
          <b>Bridge</b> {bridgeDown || bridgeWhy ? bridgeWhy || 'down.' : 'answers.'}
        </li>
      </ul>
    </section>
  {/if}
</main>

{#if sheetNode}
  {@const st = stateOf(sheetNode)}
  <div class="sheet" role="dialog" aria-label={`${sheetNode.name} detail`}>
    <h2>{sheetNode.name.toUpperCase()}</h2>
    <p class="age">{st.label}. {st.reason}</p>
    {#if sheetErr}
      <p class="err">{sheetErr}</p>
      <button class="btn ghost" type="button" onclick={() => void openSheet(sheetNode)}>Retry</button>
    {:else if sheetLines.length > 0}
      <pre>{sheetLines.slice(-20).join('\n')}</pre>
    {:else if !sheetNode.pane}
      <p class="age">No session runs, so no screen exists.</p>
    {:else}
      <p class="age">Screen loads.</p>
    {/if}
    <div class="sheetbtns">
      <a class="btn primary" href="/threads?lane=need">Answer</a>
      <a class="btn primary" href="/threads">Approve</a>
      <a class="btn primary" href="/threads">Continue</a>
      <button class="btn ghost" type="button" onclick={closeSheet}>Close</button>
    </div>
  </div>
{/if}

<style>
  main {
    max-width: 76rem;
    margin: 0 auto;
    padding: 1.5rem 1.25rem 6rem;
  }
  .secttl {
    text-align: center;
    font-size: var(--text-lg);
    font-weight: 800;
    letter-spacing: 0.2em;
    color: var(--accent);
    margin: 0;
  }
  .demo {
    color: var(--warn);
    font-size: var(--text-sm);
    text-align: center;
  }
  .age {
    color: var(--paper-dim);
    font-size: var(--text-sm);
    text-align: center;
  }
  .err {
    color: var(--bad);
    font-size: var(--text-sm);
  }
  .calmbox {
    text-align: center;
    margin-top: 3rem;
  }
  .calmbox h2 {
    font-size: var(--text-lg);
  }
  .chips {
    display: flex;
    justify-content: center;
    flex-wrap: wrap;
    gap: 0.6rem;
    margin-top: 1rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--ink-raise);
    border-radius: 999px;
    padding: 0.5rem 1rem;
    font-size: var(--text-sm);
  }
  .dot {
    width: 0.7rem;
    height: 0.7rem;
    border-radius: 50%;
    display: inline-block;
    flex: none;
  }
  .dot.live {
    animation: breathe 2s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      opacity: 0.45;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .dot.live {
      animation: none;
    }
  }
  .warn {
    margin-top: 1rem;
    border: 1px solid var(--bad);
    border-radius: 1rem;
    padding: 1rem 1.25rem;
    background: var(--ink-raise);
  }
  .warn h3 {
    margin: 0;
    font-size: var(--text-md);
  }
  .warn p {
    font-size: var(--text-sm);
  }
  .cmdline {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin: 0.5rem 0;
  }
  .cmdline code {
    background: var(--ink);
    color: var(--paper);
    border-radius: 0.6rem;
    padding: 0.4rem 0.7rem;
  }
  .picked {
    color: var(--ok);
    font-size: var(--text-sm);
  }
  .btn {
    display: inline-block;
    border-radius: 999px;
    padding: 0.7rem 1.4rem;
    font-weight: 700;
    text-decoration: none;
    border: 0;
    font-size: var(--text-sm);
    min-height: 3rem;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .btn.ghost {
    background: transparent;
    border: 1px solid var(--paper-dim);
    color: var(--paper);
  }
  .youcard {
    margin: 1.25rem auto 0;
    max-width: 24rem;
    border: 2px solid var(--accent);
    border-radius: 1.25rem;
    padding: 1.25rem;
    text-align: center;
    background: color-mix(in srgb, var(--accent) 12%, var(--ink-raise));
    box-shadow: 0 0 24px color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .youcard h2 {
    margin: 0;
    color: var(--accent);
    font-size: var(--text-lg);
  }
  .youcard ul {
    list-style: none;
    margin: 0.75rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .youcard li {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .btn-big {
    display: block;
    background: var(--accent);
    color: var(--accent-ink);
    border-radius: 999px;
    padding: 0.8rem;
    font-weight: 800;
    text-decoration: none;
    min-height: 3.25rem;
    line-height: 1.65rem;
    text-transform: uppercase;
  }
  .ribbons {
    display: block;
    width: 100%;
    max-width: 30rem;
    margin: 1rem auto 0;
  }
  .flowlanes {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-top: 1rem;
    position: relative;
  }
  .flowlanes::before {
    content: '';
    position: absolute;
    left: 0.4rem;
    top: 1rem;
    bottom: 1rem;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
    opacity: 0.55;
  }
  .donechip {
    display: block;
    margin: 1rem auto 0;
  }
  .donelist {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .donerow {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    background: var(--ink-raise);
    border: 1px solid color-mix(in srgb, var(--paper-dim) 30%, transparent);
    border-radius: 0.75rem;
    color: inherit;
    font: inherit;
    font-size: var(--text-sm);
    padding: 0.7rem 1rem;
    min-height: 3rem;
  }
  .node {
    margin-left: 1.5rem;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
    border-radius: 1rem;
    padding: 0.9rem 1rem;
    background: var(--ink-raise);
  }
  .nhead {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    background: none;
    border: 0;
    color: inherit;
    font: inherit;
    padding: 0;
    text-align: left;
  }
  .nhead strong {
    font-size: var(--text-md);
  }
  .nhead .st {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: var(--text-sm);
    color: var(--paper-dim);
    flex: none;
  }
  .nline {
    font-size: var(--text-sm);
    margin: 0.5rem 0 0;
  }
  .ns {
    font-size: var(--text-sm);
    color: var(--paper-dim);
    margin: 0.35rem 0 0;
  }
  .qbar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-top: 0.6rem;
    height: 0.6rem;
  }
  .qbar i {
    display: block;
    height: 100%;
    border-radius: 999px;
  }
  .qbar span {
    font-size: var(--text-sm);
    color: var(--paper-dim);
  }
  .nmeta {
    font-size: var(--text-sm);
    color: var(--paper-dim);
    margin: 0.35rem 0 0;
  }
  .warnline {
    font-size: var(--text-sm);
    color: var(--bad);
    margin: 0.35rem 0 0;
  }
  .legend,
  .timeline,
  .health {
    margin-top: 1.5rem;
  }
  .legend h2,
  .timeline h2,
  .health h2 {
    font-size: var(--text-sm);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--paper-dim);
  }
  .legend ul,
  .timeline ul,
  .health ul {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .legend li,
  .health li {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: var(--text-sm);
  }
  .ribbon {
    width: 1.4rem;
    height: 0.35rem;
    border-radius: 999px;
    background: var(--accent);
    flex: none;
  }
  .mdot {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: #ffd9a3;
    flex: none;
  }
  .timeline li {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    font-size: var(--text-sm);
  }
  .timeline .t {
    display: flex;
    flex-direction: column;
  }
  .timeline small {
    color: var(--paper-dim);
  }
  .sheet {
    position: fixed;
    left: 1rem;
    right: 1rem;
    bottom: calc(4.5rem + env(safe-area-inset-bottom));
    max-height: 70vh;
    overflow: auto;
    background: var(--ink-raise);
    border: 1px solid var(--accent);
    border-radius: 1.25rem;
    padding: 1.25rem;
    z-index: 20;
  }
  .sheet h2 {
    margin: 0;
    font-size: var(--text-md);
  }
  .sheet pre {
    background: var(--ink);
    border-radius: 0.75rem;
    padding: 0.75rem;
    font-size: var(--text-xs);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 16rem;
    overflow: auto;
  }
  .sheetbtns {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    margin-top: 0.9rem;
  }
  @media (min-width: 48rem) {
    .flowlanes {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
    }
    .flowlanes::before {
      display: none;
    }
    .node {
      margin-left: 0;
    }
    .sheet {
      left: auto;
      right: 2rem;
      bottom: 2rem;
      width: 28rem;
    }
  }
</style>
