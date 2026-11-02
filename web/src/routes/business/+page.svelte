<script lang="ts">
  import { onMount } from 'svelte';
  import { apiGetOr, ageLabel } from '$lib/api';

  type Kpi = { name: string; value: string };
  type Vendor = { name: string; contact: string | null };
  type Alert = { title: string; detail: string | null; level: string };
  type Overview = { kpis: Kpi[]; vendors: Vendor[]; alerts: Alert[]; updated_at: string | null };
  type Invoice = {
    id: string;
    vendor: string;
    amount_cents: number;
    currency: string;
    due: string | null;
    status: string;
  };
  type Shift = { staff: string; role: string | null; start: string; end: string };

  const demoOverview: Overview = {
    kpis: [
      { name: 'Sales today', value: '$1,240' },
      { name: 'Orders today', value: '86' },
      { name: 'Staff on shift', value: '3' }
    ],
    vendors: [
      { name: 'Bean Co', contact: 'orders@beanco.test' },
      { name: 'Cup Supply', contact: null }
    ],
    alerts: [{ title: 'Milk runs low', detail: 'Two days left at this pace.', level: 'warn' }],
    updated_at: null
  };

  const demoInvoices: Invoice[] = [
    { id: 'inv-1', vendor: 'Bean Co', amount_cents: 45000, currency: 'USD', due: '2026-11-05', status: 'open' },
    { id: 'inv-2', vendor: 'Cup Supply', amount_cents: 18250, currency: 'USD', due: null, status: 'paid' }
  ];

  const demoSchedule: Shift[] = [
    { staff: 'Sam', role: 'Barista', start: '08:00', end: '14:00' },
    { staff: 'Jo', role: 'Baker', start: '09:00', end: '15:00' }
  ];

  let overview = $state<Overview>(demoOverview);
  let invoices = $state<Invoice[]>(demoInvoices);
  let schedule = $state<Shift[]>(demoSchedule);
  let live = $state(true);
  let stale = $state(false);
  let loaded = $state(false);
  let fetchedAt = $state(new Date());

  async function load() {
    const o = await apiGetOr<Overview>('/business/overview', demoOverview);
    const i = await apiGetOr<Invoice[]>('/business/invoices', demoInvoices);
    const s = await apiGetOr<Shift[]>('/business/schedule', demoSchedule);
    overview = o.data;
    invoices = i.data;
    schedule = s.data;
    live = o.live && i.live && s.live;
    stale = o.stale || i.stale || s.stale;
    fetchedAt = o.at;
    loaded = true;
  }

  onMount(load);

  function amount(n: Invoice): string {
    const val = (n.amount_cents / 100).toFixed(2);
    return `${n.currency} ${val}`;
  }
</script>

<main>
  <h1>Business</h1>
  <img class="mast" src="/art/module-business.webp" alt="" width="1536" height="1020" />
  {#if !live && stale}<p class="demo">Last live data. Start the server to refresh.</p>
  {:else if !live}<p class="demo">Sample data. Start the server to see live data.</p>{/if}
  {#if loaded}
    <p class="age">Seen {ageLabel(fetchedAt)}</p>
    <section class="card">
      <h2>Key numbers</h2>
      <div class="kpis">
        {#each overview.kpis as k (k.name)}
          <div class="kpi">
            <p class="num">{k.value}</p>
            <p class="name">{k.name}</p>
          </div>
        {/each}
      </div>
      {#if overview.kpis.length === 0}<p>No numbers yet.</p>{/if}
    </section>
    <section class="card">
      <h2>Notices</h2>
      {#each overview.alerts as a (a.title)}
        <p class:warn={a.level === 'warn'}>{a.title}{a.detail ? ` — ${a.detail}` : ''}</p>
      {/each}
      {#if overview.alerts.length === 0}<p>No notices.</p>{/if}
    </section>
    <section class="card">
      <h2>Bills</h2>
      {#each invoices as v (v.id)}
        <div class="line">
          <p>{v.vendor} · {amount(v)} · {v.status}{v.due ? ` · due ${v.due}` : ''}</p>
        </div>
      {/each}
      {#if invoices.length === 0}<p>No bills.</p>{/if}
    </section>
    <section class="card">
      <h2>Shifts</h2>
      {#each schedule as s (s.staff + s.start)}
        <p>{s.staff}{s.role ? ` (${s.role})` : ''} · {s.start}–{s.end}</p>
      {/each}
      {#if schedule.length === 0}<p>No shifts.</p>{/if}
    </section>
    <section class="card">
      <h2>Vendors</h2>
      {#each overview.vendors as v (v.name)}
        <p>{v.name}{v.contact ? ` · ${v.contact}` : ''}</p>
      {/each}
      {#if overview.vendors.length === 0}<p>No vendors.</p>{/if}
    </section>
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
  .kpis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
    gap: 0.75rem;
  }
  .kpi {
    background: var(--ink);
    border-radius: 0.75rem;
    padding: 0.75rem 1rem;
  }
  .kpi .num {
    font-size: var(--text-lg);
    font-variant-numeric: var(--nums);
    margin: 0;
  }
  .kpi .name {
    font-size: var(--text-sm);
    color: var(--paper-dim);
    margin: 0;
  }
  .warn {
    color: var(--warn);
  }
  .line p,
  .card p {
    margin: 0.25rem 0;
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
