<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, ageLabel } from '$lib/api';
  import { segments, type TodayView } from '$lib/today';
  import { demoToday } from '$lib/demo';

  let view = $state<TodayView | null>(null);
  let at = $state<Date | null>(null);
  let live = $state(true);
  let stale = $state(false);
  let bloomed = $state(false);

  onMount(() => {
    // Pause ambient loops when the tab hides. It saves power.
    const onVis = () => {
      document.body.classList.toggle('tab-hidden', document.hidden);
      document.querySelectorAll<HTMLVideoElement>('main video').forEach((v) => {
        if (document.hidden) v.pause();
        else void v.play().catch(() => {});
      });
    };
    document.addEventListener('visibilitychange', onVis);
    apiGetOr<TodayView>('/today', demoToday).then((res) => {
      view = res.data;
      at = res.at;
      live = res.live;
      stale = res.stale;
      requestAnimationFrame(() => (bloomed = true));
    });
    return () => document.removeEventListener('visibilitychange', onVis);
  });

  const segs = $derived(view ? segments(view) : []);
  const N = 5;
  const LEN = 100 / N;
  const GAP = 2;

  const hour = new Date().getHours();
  const greeting = hour < 12 ? 'Good morning.' : hour < 18 ? 'Good afternoon.' : 'Good evening.';
  const dateLine = new Date().toLocaleDateString(undefined, {
    weekday: 'long',
    month: 'long',
    day: 'numeric'
  });
</script>

<main>
  <video
    class="bg"
    src="/art/bg-ambient-today.mp4"
    poster="/art/bg-ambient-still.webp"
    autoplay
    muted
    loop
    playsinline
    width="1276"
    height="720"
    aria-hidden="true"
  ></video>
  <img
    class="bg-still"
    src="/art/bg-ambient-still.webp"
    alt=""
    width="1920"
    height="1086"
    aria-hidden="true"
  />
  <p class="eyebrow">Hearth · {dateLine}</p>
  {#if view}
    {#if !live && stale}<p class="demo">Last live data {at ? ageLabel(at) : ''}.</p>
    {:else if !live}<p class="demo">Sample data. Start the server for live data.</p>{/if}
    <div class="hero">
      <div class="wheel" class:bloomed role="img" aria-label="Life wheel: state of each area now">
        <svg viewBox="0 0 100 100">
          {#each segs as s, i}
            <circle
              cx="50"
              cy="50"
              r="40"
              fill="none"
              stroke={s.hue}
              stroke-width={s.count > 0 ? 7 : 3}
              stroke-dasharray={`${LEN - GAP} ${100 - LEN + GAP}`}
              stroke-dashoffset={-i * LEN + 25}
              stroke-linecap="round"
              style="--i: {i}; --end-opacity: {s.count > 0 ? 1 : 0.3}"
            />
          {/each}
        </svg>
        <div class="avatar" aria-hidden="true">
          <video
            class="motion"
            src="/art/avatar-ember-idle.mp4"
            poster="/art/avatar-ember-still.webp"
            autoplay
            muted
            loop
            playsinline
            width="960"
            height="960"
          ></video>
          <img class="still" src="/art/avatar-ember-still.webp" alt="" width="512" height="512" />
        </div>
      </div>
      <div class="now">
        <h1>{greeting}</h1>
        <p class="age num">{at ? ageLabel(at) : ''}</p>
        {#if view.next_action}
          <section class="card next">
            <h2>Next</h2>
            <p class="next-title">{view.next_action.title}</p>
          </section>
        {:else}
          <section class="card calm">
            <h2>All calm.</h2>
            <p>Nothing needs you now.</p>
          </section>
        {/if}
      </div>
    </div>
    <div class="cards">
      <section class="card">
        <h2>Agenda <span class="num">{view.agenda.length}</span></h2>
        {#each view.agenda.slice(0, 3) as a}
          <p>{a.title}</p>
        {/each}
      </section>
      <section class="card">
        <h2>Due <span class="num">{view.reminders_due.length + view.tasks_due.length}</span></h2>
        {#each [...view.reminders_due, ...view.tasks_due].slice(0, 3) as d}
          <p>{d.title}</p>
        {/each}
      </section>
      <section class="card">
        <h2>Health</h2>
        <p>{view.health_checkin_done ? 'Check-in done.' : 'Check-in waits (10 sec).'}</p>
        {#if !view.health_checkin_done}
          <a class="go" href="/health">Check in</a>
        {/if}
      </section>
    </div>
    <div class="soon">
      <section class="card" aria-label="Insights, soon">
        <h2>Insights <span class="badge">Soon</span></h2>
        <div class="shimmer"></div>
        <div class="shimmer short"></div>
      </section>
      <section class="card" aria-label="This week, soon">
        <h2>This week <span class="badge">Soon</span></h2>
        <div class="dots">
          {#each { length: 7 } as _, i}
            <span class:lit={i < 3}></span>
          {/each}
        </div>
      </section>
    </div>
  {:else}
    <!-- Loading state keeps the shape of the real page. -->
    <div class="hero" aria-busy="true" aria-label="Today loads">
      <div class="wheel rest">
        <div class="ring"></div>
        <div class="core"></div>
      </div>
      <div class="now">
        <div class="bar title"></div>
        <div class="bar age"></div>
        <div class="ghost-next"></div>
      </div>
    </div>
    <div class="cards">
      {#each { length: 3 } as _}
        <section class="card ghost">
          <div class="bar"></div>
          <div class="bar"></div>
          <div class="bar short"></div>
        </section>
      {/each}
    </div>
  {/if}
</main>

<style>
  main {
    /* No stacking context here: the avatar screen-blend must reach the page. */
    max-width: 64rem;
    margin: 0 auto;
    padding: 2.5rem 1.5rem 5rem;
  }
  /* Ambient film behind Today. Dark theme only; light keeps clean paper. */
  .bg,
  .bg-still {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    z-index: -1;
    pointer-events: none;
    filter: brightness(1.3);
    -webkit-mask-image: linear-gradient(to bottom, #000 45%, transparent 80%);
    mask-image: linear-gradient(to bottom, #000 45%, transparent 80%);
  }
  .bg-still {
    display: none;
  }
  :global([data-theme='light']) .bg,
  :global([data-theme='light']) .bg-still {
    display: none;
  }
  .eyebrow {
    color: var(--paper-dim);
    font-size: var(--text-sm);
    margin: 0;
  }
  .age {
    color: var(--paper-dim);
    font-size: var(--text-sm);
    margin: 0.25rem 0 0;
  }
  .demo {
    color: var(--accent);
    font-size: var(--text-sm);
    margin: 0.5rem 0 0;
  }
  .hero {
    display: grid;
    gap: 2rem;
    align-items: center;
    margin-top: 1rem;
  }
  @media (min-width: 56rem) {
    .hero {
      grid-template-columns: minmax(0, 5fr) minmax(0, 7fr);
    }
  }
  .wheel {
    position: relative;
    width: min(22rem, 80vw);
    margin: 0 auto;
  }
  /* Ambient glow drifts behind the wheel. Slow, low, calm. */
  .wheel::before {
    content: '';
    position: absolute;
    inset: -12%;
    background:
      radial-gradient(closest-side, rgba(232, 160, 76, 0.35), transparent 70%),
      radial-gradient(closest-side, rgba(232, 160, 76, 0.14), transparent 70%);
    background-size: 58% 58%, 100% 100%;
    background-position: center;
    background-repeat: no-repeat;
    pointer-events: none;
    animation: drift 12s var(--ease-out) infinite alternate;
  }
  @keyframes drift {
    from {
      transform: scale(1);
      opacity: 0.75;
    }
    to {
      transform: scale(1.12);
      opacity: 1;
    }
  }
  .wheel svg {
    width: 100%;
    transform: rotate(-90deg);
    position: relative;
  }
  .wheel circle {
    opacity: 0;
  }
  .wheel.bloomed circle {
    animation: bloom var(--dur-slow) var(--ease-spring) forwards;
    animation-delay: calc(var(--i, 0) * 90ms);
  }
  @keyframes bloom {
    from {
      opacity: 0;
      transform: scale(0.6);
      transform-origin: center;
    }
    to {
      opacity: var(--end-opacity, 1);
      transform: scale(1);
    }
  }
  /* Bloom ends at each ring's own opacity (set inline). */
  .avatar {
    position: absolute;
    inset: 0;
    margin: auto;
    width: 54%;
    height: 54%;
    border-radius: 50%;
    mix-blend-mode: screen;
    animation: breathe 4.5s var(--ease-out) infinite;
    /* Feather the square art into a glow. No flat disc edge. */
    -webkit-mask-image: radial-gradient(closest-side, #000 52%, transparent 70%);
    mask-image: radial-gradient(closest-side, #000 52%, transparent 70%);
    filter: drop-shadow(0 0 1rem rgba(232, 160, 76, 0.5));
    overflow: hidden;
  }
  .avatar .motion,
  .avatar .still {
    /* Zoom the frame: the flame is small in its art, fill the box with it. */
    position: absolute;
    left: 50%;
    top: 50%;
    width: 220%;
    height: 220%;
    transform: translate(-50%, -50%);
    border-radius: 50%;
    display: block;
  }
  .avatar .still {
    display: none;
  }
  @keyframes breathe {
    0%,
    100% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.045);
    }
  }
  .now h1 {
    font-size: var(--text-display);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .cards {
    display: grid;
    gap: 1rem;
    margin-top: 1.5rem;
  }
  @media (min-width: 40rem) {
    .cards {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (min-width: 56rem) {
    .cards {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  .card {
    background: var(--ink-raise);
    border-radius: 1rem;
    padding: 1.25rem 1.5rem;
  }
  .card h2 {
    font-size: var(--text-md);
    margin: 0 0 0.5rem;
  }
  .card p {
    margin: 0.25rem 0;
  }
  .next {
    border: 1px solid var(--accent);
    margin-top: 1.25rem;
  }
  .next-title {
    font-size: var(--text-lg);
  }
  .calm {
    margin-top: 1.25rem;
  }
  .go {
    display: inline-block;
    margin-top: 0.5rem;
    color: var(--accent);
    text-decoration: none;
  }
  .soon {
    display: grid;
    gap: 1rem;
    margin-top: 1rem;
  }
  @media (min-width: 56rem) {
    .soon {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .badge {
    font-family: var(--font-ui);
    font-size: var(--text-xs);
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 999px;
    padding: 0.1rem 0.6rem;
    margin-left: 0.5rem;
    vertical-align: middle;
  }
  .shimmer {
    height: 0.9rem;
    border-radius: 0.5rem;
    margin: 0.6rem 0 0;
    background: linear-gradient(90deg, var(--ink) 25%, var(--paper-dim) 50%, var(--ink) 75%);
    background-size: 200% 100%;
    opacity: 0.5;
    animation: sweep 1.8s linear infinite;
  }
  .shimmer.short {
    width: 55%;
  }
  @keyframes sweep {
    from {
      background-position: 200% 0;
    }
    to {
      background-position: -200% 0;
    }
  }
  .dots {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }
  .dots span {
    width: 0.9rem;
    height: 0.9rem;
    border-radius: 50%;
    background: var(--ink);
    border: 1px solid var(--paper-dim);
  }
  .dots .lit {
    background: var(--accent);
    border-color: var(--accent);
    opacity: 0.7;
  }
  /* Light theme: tight mask keeps a small ember heart; halo sits behind it. */
  :global([data-theme='light']) .avatar {
    mix-blend-mode: normal;
    -webkit-mask-image: radial-gradient(closest-side, #000 42%, transparent 60%);
    mask-image: radial-gradient(closest-side, #000 42%, transparent 60%);
    filter: drop-shadow(0 0 1.4rem rgba(232, 160, 76, 0.55));
  }
  :global([data-theme='light']) .wheel::before {
    background:
      radial-gradient(closest-side, rgba(232, 160, 76, 0.45), transparent 70%),
      radial-gradient(closest-side, rgba(232, 160, 76, 0.14), transparent 70%);
    background-size: 62% 62%, 100% 100%;
    background-position: center;
    background-repeat: no-repeat;
  }
  /* Loading shapes. Token colours only, no art yet. */
  .rest .ring {
    aspect-ratio: 1;
    border-radius: 50%;
    border: 0.7rem solid var(--ink-raise);
  }
  .rest .core {
    position: absolute;
    inset: 0;
    margin: auto;
    width: 26%;
    height: 26%;
    border-radius: 50%;
    background: var(--ink-raise);
  }
  .rest::before {
    animation: none;
  }
  .bar {
    height: 1rem;
    border-radius: 0.5rem;
    background: var(--paper-dim);
    opacity: 0.35;
    margin: 0.5rem 0;
  }
  .bar.title {
    height: 2.5rem;
    width: 70%;
  }
  .bar.age {
    width: 25%;
  }
  .bar.short {
    width: 40%;
  }
  .ghost {
    min-height: 8rem;
  }
  .ghost-next {
    height: 5.5rem;
    border-radius: 1rem;
    border: 1px solid var(--paper-dim);
    opacity: 0.5;
    margin-top: 1.25rem;
  }
  /* Phones: small wheel, so Next shows without a scroll. */
  @media (max-width: 40rem) {
    main {
      padding-top: 1.5rem;
    }
    .hero {
      gap: 1rem;
    }
    .wheel {
      width: min(11rem, 46vw);
    }
    .now h1 {
      font-size: var(--text-xl);
    }
  }
  /* Hidden tab: freeze ambient loops. */
  :global(.tab-hidden) .wheel::before,
  :global(.tab-hidden) .avatar,
  :global(.tab-hidden) .shimmer {
    animation-play-state: paused;
  }
  @media (prefers-reduced-motion: reduce) {
    /* No bloom, no drift, no film. Rings crossfade in gently instead. */
    .avatar .motion,
    .bg {
      display: none;
    }
    .avatar .still {
      display: block;
    }
    .bg-still {
      display: block;
    }
    :global([data-theme='light']) .bg-still {
      display: none;
    }
    .wheel.bloomed circle {
      animation: fadein 160ms linear forwards;
      animation-delay: calc(var(--i, 0) * 40ms);
    }
    @keyframes fadein {
      from {
        opacity: 0;
      }
      to {
        opacity: var(--end-opacity, 1);
      }
    }
    .wheel::before,
    .avatar,
    .shimmer {
      animation: none;
    }
  }
</style>
