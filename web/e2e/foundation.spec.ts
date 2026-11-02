import { test, expect } from '@playwright/test';

const routes = [
  { path: '/', name: 'today' },
  { path: '/today', name: 'today-classic' },
  { path: '/dev', name: 'dev' },
  { path: '/reminders', name: 'reminders' },
  { path: '/tasks', name: 'tasks' },
  { path: '/inventory', name: 'inventory' },
  { path: '/health', name: 'health' },
  { path: '/business', name: 'business' },
  { path: '/factory', name: 'factory' },
  { path: '/builds', name: 'builds' },
  { path: '/threads', name: 'threads' },
  { path: '/needs-you', name: 'needs-you' },
  { path: '/features', name: 'features' },
  { path: '/settings', name: 'settings' }
];

test('shell uses tokens, not browser defaults', async ({ page }) => {
  await page.goto('/today');
  await expect(page.getByRole('img', { name: /Life wheel/ })).toBeVisible();
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  const font = await page.evaluate(() => getComputedStyle(document.body).fontFamily);
  expect(bg).not.toBe('rgb(255, 255, 255)');
  expect(bg).toBe('rgb(20, 16, 12)');
  expect(font).toContain('system-ui');
});

test('demo Today renders fully with zero failed requests', async ({ page }) => {
  const failed: string[] = [];
  page.on('requestfailed', (r) => failed.push(r.url()));
  page.on('response', (r) => {
    if (r.status() >= 400) failed.push(`${r.status()} ${r.url()}`);
  });
  await page.goto('/');
  await expect(page.getByRole('img', { name: /Today,/ })).toBeVisible();
  await expect(page.getByText('Sample data')).toBeVisible();
  await expect(page.getByText('Do this next')).toBeVisible();
  expect(failed).toEqual([]);
});

test('reduced motion crossfades the wheel in', async ({ browser }) => {
  const ctx = await browser.newContext({ reduced_motion: 'reduce' });
  const page = await ctx.newPage();
  await page.goto('/today?theme=dark');
  await expect(page.getByRole('img', { name: /Life wheel/ })).toBeVisible();
  await page.waitForTimeout(500);
  const op = await page.evaluate(() => {
    const c = document.querySelector('.wheel circle');
    return c ? getComputedStyle(c).opacity : 'missing';
  });
  expect(Number(op)).toBeGreaterThan(0);
  await ctx.close();
});

for (const theme of ['dark', 'light']) {
  for (const r of routes) {
    for (const width of [390, 1440]) {
      test(`shot ${r.name} ${theme} ${width}`, async ({ page }) => {
        await page.setViewportSize({ width, height: Math.round((width * 16) / 9) });
        await page.goto(`${r.path}?theme=${theme}`);
        await page.waitForTimeout(900);
        await page.screenshot({ path: `e2e/shots/${r.name}-${theme}-${width}.png` });
      });
    }
  }
}
