/** Today view types. Mirror the Rust TodayView (snake_case). */

export type Item = { id: string; title: string; at: string | null };

export type TodayView = {
  agenda: Item[];
  reminders_due: Item[];
  tasks_due: Item[];
  business_alerts: Item[];
  health_checkin_done: boolean;
  next_action: Item | null;
  updated_at: string | null;
};

export type Segment = {
  key: string;
  label: string;
  count: number;
  done: boolean;
  hue: string;
};

/** Wheel segments from a view. One per module area. */
export function segments(view: TodayView): Segment[] {
  return [
    { key: 'agenda', label: 'Agenda', count: view.agenda.length, done: false, hue: 'var(--hue-today)' },
    { key: 'reminders', label: 'Reminders', count: view.reminders_due.length, done: false, hue: 'var(--hue-reminders)' },
    { key: 'tasks', label: 'Tasks', count: view.tasks_due.length, done: false, hue: 'var(--hue-tasks)' },
    { key: 'business', label: 'Business', count: view.business_alerts.length, done: false, hue: 'var(--hue-business)' },
    { key: 'health', label: 'Health', count: view.health_checkin_done ? 0 : 1, done: view.health_checkin_done, hue: 'var(--hue-health)' }
  ];
}
