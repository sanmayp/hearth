/** Sample data. Shows when the server stays quiet. Never real data. */
import type { TodayView } from './today';

export const demoToday: TodayView = {
  agenda: [
    { id: 'a1', title: 'Fake dentist visit', at: '2026-11-01T09:00:00Z' },
    { id: 'a2', title: 'Fake call with Sam', at: '2026-11-01T14:00:00Z' }
  ],
  reminders_due: [{ id: 'r1', title: 'Fake: take vitamins', at: '2026-10-31T08:00:00Z' }],
  tasks_due: [
    { id: 't1', title: 'Fake: water the plants', at: null },
    { id: 't2', title: 'Fake: read 10 pages', at: null }
  ],
  business_alerts: [],
  health_checkin_done: false,
  next_action: { id: 't1', title: 'Fake: water the plants', at: null },
  updated_at: null
};

export type DemoReminder = {
  id: string;
  title: string;
  at: string | null;
  rrule: string | null;
  done: boolean;
  snoozed_until: string | null;
};

export const demoReminders: DemoReminder[] = [
  { id: 'r1', title: 'Fake: take vitamins', at: '2026-10-31T08:00:00Z', rrule: 'RRULE:FREQ=DAILY', done: false, snoozed_until: null },
  { id: 'r2', title: 'Fake: call the bank', at: null, rrule: null, done: false, snoozed_until: null }
];

export type DemoTask = {
  id: string;
  title: string;
  list: 'inbox' | 'today' | 'someday';
  project: string | null;
  done: boolean;
  steps: { title: string; done: boolean }[];
};

export const demoTasks: DemoTask[] = [
  { id: 't1', title: 'Fake: water the plants', list: 'today', project: null, done: false, steps: [] },
  { id: 't2', title: 'Fake: plan trip then book flight', list: 'inbox', project: null, done: false, steps: [{ title: 'Fake: plan trip', done: true }, { title: 'Fake: book flight', done: false }] },
  { id: 't3', title: 'Fake: learn salsa', list: 'someday', project: null, done: false, steps: [] }
];

export type DemoItem = {
  id: string;
  name: string;
  room: string;
  barcode: string | null;
  note: string | null;
};

export const demoItems: DemoItem[] = [
  { id: 'i1', name: 'Fake passport', room: 'study', barcode: null, note: 'top drawer' },
  { id: 'i2', name: 'Fake spare key', room: 'kitchen', barcode: null, note: null }
];

export const demoRooms: string[] = ['study', 'kitchen'];

export type DemoCheckin = {
  id: string;
  day: string;
  payload: { mood: number; energy: number; note: string | null };
  at: string;
};

export const demoCheckins: DemoCheckin[] = [
  { id: 'c1', day: '2026-10-30', payload: { mood: 3, energy: 2, note: 'Fake: short night.' }, at: '2026-10-30T08:00:00Z' }
];

export type BuildStage = 'made' | 'tested' | 'live' | 'checked' | 'done';

export type DemoBuild = {
  id: string;
  name: string;
  stage: BuildStage;
  note: string;
};

export const demoBuilds: DemoBuild[] = [
  { id: 'b1', name: 'Hearth UI step 3', stage: 'tested', note: 'Shots wait on owner eyes.' },
  { id: 'b2', name: 'Tank glance app', stage: 'made', note: 'Build runs, tests fail.' },
  { id: 'b3', name: 'Nightly data pulls', stage: 'live', note: 'Filings land nightly.' }
];

export type DemoAgent = {
  id: string;
  name: string;
  state: 'working' | 'idle' | 'blocked' | 'done';
  job: string;
};

export type MuseSession = {
  name: string;
  pane: string;
  state: string;
  at: string | null;
  changed_at: string | null;
  alerts: string[];
  stage: string | null;
  question: string | null;
  recommendation: string | null;
};

export type MuseBoard = {
  sessions: MuseSession[];
  log_at: string | null;
};

export const demoMuse: MuseBoard = {
  sessions: [
    { name: 'lifeos', pane: 'w1:p19', state: 'working', at: null, changed_at: null, alerts: [], stage: null, question: null, recommendation: null },
    { name: 'muse-trade', pane: 'w1:p1X', state: 'idle', at: null, changed_at: null, alerts: [], stage: null, question: null, recommendation: null },
    { name: 'muse-rummy-engine', pane: 'w1:p1Z', state: 'idle', at: null, changed_at: null, alerts: [], stage: null, question: null, recommendation: null },
    { name: 'muse-station', pane: 'w1:p21', state: 'idle', at: null, changed_at: null, alerts: [], stage: null, question: null, recommendation: null },
    { name: 'muse-supervisor', pane: 'w1:p22', state: 'idle', at: null, changed_at: null, alerts: [], stage: null, question: null, recommendation: null }
  ],
  log_at: null
};

export type FactorySection = { title: string; body: string };

export type FactoryBuild = { id: string; name: string; stage: string; note: string };

export type FactoryRun = {
  ts: string;
  lane: string;
  work: string;
  outcome: string;
};

export type FactoryStatus = {
  state: { sections: FactorySection[]; raw: string | null } | null;
  builds: FactoryBuild[] | null;
  ledger: FactoryRun[];
};

export const demoFactory: FactoryStatus = {
  state: {
    sections: [
      { title: 'In flight', body: 'Sample lane ships.' },
      { title: 'Merged', body: 'Sample lane done.' }
    ],
    raw: null
  },
  builds: [{ id: 'b1', name: 'Sample build', stage: 'made', note: 'Sample row.' }],
  ledger: [{ ts: '', lane: 'sample', work: 'Sample work.', outcome: 'Sample outcome.' }]
};

export const demoAgents: DemoAgent[] = [
  { id: 'a1', name: 'muse (lifeos)', state: 'working', job: 'Builds board plus voice.' },
  { id: 'a2', name: 'helper (site)', state: 'idle', job: 'Waits at prompt.' },
  { id: 'a3', name: 'muse (home)', state: 'idle', job: 'Waits at prompt.' },
  { id: 'a4', name: 'muse (tank)', state: 'working', job: 'Fixes test failures.' }
];

export type ThreadFlow = {
  name: string;
  pane: string;
  state: string;
  flow: 'need' | 'building' | 'paused' | 'review' | 'waiting';
  at: string | null;
  changed_at: string | null;
  line: string;
  stage: string | null;
  question: string | null;
  recommendation: string | null;
};

export type NeedAnswer = {
  thread: string;
  question: string;
  recommendation: string | null;
};

export type ThreadBoard = {
  threads: ThreadFlow[];
  needs_answer: NeedAnswer[];
  log_at: string | null;
  generated_at?: string;
};

export type AskEntry = {
  at: string;
  thread: string | null;
  text: string;
};

export type AskQueue = {
  entries: AskEntry[];
};

export const demoThreads: ThreadBoard = {
  threads: [
    { name: 'lifeos', pane: 'w1:p19', state: 'working', flow: 'building', at: null, changed_at: null, line: 'Builds in the background.', stage: null, question: null, recommendation: null },
    { name: 'muse-trade', pane: 'w1:p1X', state: 'blocked', flow: 'need', at: null, changed_at: null, line: 'Waits on the owner answer.', stage: null, question: 'Shows an approval prompt. Needs the owner answer to continue.', recommendation: null },
    { name: 'muse-rummy-engine', pane: 'w1:p1Z', state: 'idle', flow: 'waiting', at: null, changed_at: null, line: 'Waits for the next task.', stage: null, question: null, recommendation: null },
    { name: 'muse-station', pane: 'w1:p21', state: 'done', flow: 'review', at: null, changed_at: null, line: 'Ready for review.', stage: null, question: null, recommendation: null },
    { name: 'muse-supervisor', pane: 'w1:p22', state: 'gone', flow: 'paused', at: null, changed_at: null, line: 'Session gone.', stage: null, question: null, recommendation: null }
  ],
  needs_answer: [
    { thread: 'muse-trade', question: 'Shows an approval prompt. Needs the owner answer to continue.', recommendation: null }
  ],
  log_at: null
};

export const demoAskQueue: AskQueue = { entries: [] };
