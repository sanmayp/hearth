import { describe, expect, it } from 'vitest';
import type { FeatureRow, FeatureStep } from './features';
import type { NeedsYouFeed, NeedThread } from './needs-you';
import { pickNext } from './nextstep';

function step(over: Partial<FeatureStep> = {}): FeatureStep {
  return { text: 'Do the thing.', owner: 'you', done: false, ...over };
}

function row(steps: FeatureStep[], over: Partial<FeatureRow> = {}): FeatureRow {
  return {
    id: 'f1',
    name: 'Fix install',
    area: 'Installer',
    stage: 'built',
    where: 'Needs You',
    tested: '',
    deployed: '',
    last_used: '',
    next: '',
    steps,
    ...over
  };
}

function thread(over: Partial<NeedThread> = {}): NeedThread {
  return {
    name: 'agent a',
    pane: 'p1',
    state: 'working',
    at: null,
    changed_at: null,
    alerts: [],
    question: null,
    ...over
  };
}

function feed(over: Partial<NeedsYouFeed> = {}): NeedsYouFeed {
  return { threads: [], unused: [], events: [], log_at: null, ...over };
}

describe('pickNext', () => {
  it('routes a screen step to that screen', () => {
    const s = pickNext(
      feed(),
      [row([step({ text: 'Open the Needs You page.', where: 'Phone > Hearth > Needs You' })])],
      {}
    );
    expect(s?.ctaHref).toBe('/needs-you');
    expect(s?.ctaLabel).toBe('Go');
    expect(s?.openChecklist).toBe(false);
  });

  it('routes reply-like text to Threads', () => {
    const s = pickNext(
      feed(),
      [row([step({ text: 'Send the feature reply.', where: 'Termius' })])],
      {}
    );
    expect(s?.ctaHref).toBe('/threads');
    expect(s?.ctaLabel).toBe('Respond');
  });

  it('falls back to the Features page for shell steps', () => {
    const s = pickNext(
      feed(),
      [row([step({ text: 'Install the APK.', where: 'Termius', cmd: 'adb install app.apk' })])],
      {}
    );
    expect(s?.ctaHref).toBe('/features');
    expect(s?.ctaLabel).toBe('See the step');
    expect(s?.copyText).toContain('Run: adb install app.apk');
  });

  it('skips done steps and uses the first open one', () => {
    const s = pickNext(
      feed(),
      [
        row([
          step({ text: 'Old step.', done: true, where: 'Needs You' }),
          step({ text: 'Next step.', where: 'Settings > Bridge key' })
        ])
      ],
      {}
    );
    expect(s?.name).toBe('Next step.');
    expect(s?.ctaHref).toBe('/settings');
  });

  it('prefers you-steps over threads with questions', () => {
    const s = pickNext(
      feed({ threads: [thread({ question: 'Which port?' })] }),
      [row([step({ text: 'Open the Needs You page.', where: 'Needs You' })])],
      {}
    );
    expect(s?.name).toBe('Open the Needs You page.');
  });

  it('routes a waiting question to Threads', () => {
    const s = pickNext(feed({ threads: [thread({ question: 'Which port?' })] }), [], {});
    expect(s?.ctaHref).toBe('/threads');
    expect(s?.ctaLabel).toBe('Respond');
    expect(s?.copyText).toContain('Which port?');
  });

  it('routes a wait claim to the strip anchor', () => {
    const s = pickNext(
      feed({ threads: [thread({ state: 'idle', changed_at: '2026-10-01 10:00' })] }),
      [],
      {}
    );
    expect(s?.ctaHref).toBe('#your-move');
    expect(s?.ctaLabel).toBe('Read the claim');
  });

  it('opens the checklist for the first unopened screen', () => {
    const s = pickNext(
      feed({
        unused: [{ id: 'u1', name: 'Builds', route: '/builds', shipped_at: '', note: 'New.' }]
      }),
      [],
      {}
    );
    expect(s?.ctaHref).toBe('#unused-check');
    expect(s?.ctaLabel).toBe('Open checklist');
    expect(s?.openChecklist).toBe(true);
  });

  it('skips opened screens in the checklist fallback', () => {
    const s = pickNext(
      feed({
        unused: [
          { id: 'u1', name: 'Builds', route: '/builds', shipped_at: '', note: '' },
          { id: 'u2', name: 'Prompts', route: '/prompts', shipped_at: '', note: '' }
        ]
      }),
      [],
      { '/builds': { first: 'a', last: 'b' } }
    );
    expect(s?.name).toContain('Prompts');
  });

  it('returns null when Home holds nothing', () => {
    expect(pickNext(feed(), [], {})).toBeNull();
  });
});
