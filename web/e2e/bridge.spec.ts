import { test, expect } from '@playwright/test';

/**
 * Bridge control states at phone width. The bridge API is mocked
 * with route interception; the UI flow is real. Shots land in
 * e2e/shots/. The ?bridge= seam only accepts loopback bases.
 */
const BRIDGE = 'http://127.0.0.1:4174';

async function mockBridge(page) {
  await page.addInitScript(() => {
    localStorage.setItem('hearth:bridge-key', 'test-key');
  });
  await page.route('**/v1/panes/*/message/preview*', async (route) => {
    const url = new URL(route.request().url());
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        code: 'AB12',
        text: url.searchParams.get('text') ?? '',
        pane: 'w1:p19',
        expires_in: 60,
        approval_waiting: false,
        approval_prompt: null
      })
    });
  });
  await page.route('**/v1/panes/*/message', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ sent: true, pane: 'w1:p19' })
    });
  });
  await page.route('**/v1/panes', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ panes: [] })
    });
  });
}

test.use({ viewport: { width: 390, height: 693 } });

test('bridge down state', async ({ page }) => {
  // Key present, link dead: nothing listens on port 9.
  await page.addInitScript(() => {
    localStorage.setItem('hearth:bridge-key', 'test-key');
  });
  await page.goto(
    '/needs-you?theme=dark&bridge=' + encodeURIComponent('http://127.0.0.1:9'),
    { waitUntil: 'networkidle' }
  );
  const note = page.getByText('Bridge is down. Check the link.');
  await expect(note).toBeVisible();
  await note.scrollIntoViewIfNeeded();
  await page.screenshot({ path: 'e2e/shots/needs-you-bridge-down-390.png' });
});

test('answer flow states', async ({ page }) => {
  await mockBridge(page);
  await page.goto('/needs-you?theme=dark&bridge=' + encodeURIComponent(BRIDGE), {
    waitUntil: 'networkidle'
  });
  const card = page.locator('.tcard').first();
  await card.getByRole('button', { name: 'Answer' }).click();
  await card.getByLabel('Answer text').fill('ship it');
  await card.getByRole('button', { name: 'Preview' }).click();
  await expect(card.getByLabel('Confirm code')).toBeVisible();
  await card.scrollIntoViewIfNeeded();
  await page.screenshot({ path: 'e2e/shots/needs-you-bridge-preview-390.png' });
  await card.getByLabel('Type the shown code').fill('AB12');
  await page.screenshot({ path: 'e2e/shots/needs-you-bridge-confirm-390.png' });
  await card.getByRole('button', { name: 'Confirm' }).click();
  await expect(card.getByText('Sent.')).toBeVisible();
  await page.screenshot({ path: 'e2e/shots/needs-you-bridge-success-390.png' });
});
