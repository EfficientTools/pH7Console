import { test, expect } from '@playwright/test';

test.describe('pH7Console desktop shell', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'pH7Console' })).toBeVisible();
  });

  test('renders the primary desktop regions', async ({ page }) => {
    await expect(page.getByRole('tab', { name: 'Terminals' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Explorer' })).toBeVisible();
    await expect(page.getByText('Start a terminal session', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: /Start Terminal|Retry Terminal/ })).toBeEnabled();
    await expect(page.getByRole('heading', { name: 'Local Command Intelligence' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Settings' })).toBeVisible();
  });

  test('toggles the sidebar and AI panel with desktop controls', async ({ page }) => {
    await page.getByRole('button', { name: 'Hide Sidebar (⌘B)' }).click();
    await expect(page.getByRole('heading', { name: 'pH7Console' })).toBeHidden();

    await page.getByRole('button', { name: 'Hide AI Panel (⌘J)' }).click();
    await expect(page.getByRole('heading', { name: 'Local Command Intelligence' })).toBeHidden();
  });

  test('applies the settings it exposes', async ({ page }) => {
    await page.getByRole('button', { name: 'Settings' }).click();
    await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
    await expect(page.getByText('Font Size: 14px')).toBeVisible();

    await page.locator('input[type="range"]').fill('18');
    await expect(page.getByText('Font Size: 18px')).toBeVisible();
    await expect
      .poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue('--terminal-font-size')))
      .toBe('18px');

    await page.getByRole('tab', { name: 'Keyboard' }).click();
    await expect(page.getByText('Cmd+T')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Edit' })).toHaveCount(0);

    await page.getByRole('button', { name: 'Close settings' }).click();
    await expect(page.getByRole('heading', { name: 'Settings' })).toBeHidden();
  });

  test('reports unavailable native AI honestly in a browser preview', async ({ page }) => {
    await expect(page.getByText('Start a terminal session to create a command plan.')).toBeVisible();
    await expect(page.getByRole('textbox', { name: 'Describe a command to plan' })).toBeEnabled();
    await expect(page.getByRole('button', { name: 'Retry AI' })).toBeVisible();
    await expect(page.getByText('AI Ready')).toHaveCount(0);
  });
});

test('keeps controls readable and reachable at the minimum release window size', async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto('/');
  await expect(page.locator('header').getByText('pH7Console', { exact: true })).toBeVisible();

  await expect(page.getByRole('heading', { name: 'Local Command Intelligence' })).toBeHidden();
  await expect(page.getByRole('button', { name: 'Show Sidebar (⌘B)' })).toBeVisible();
  await expect(page.getByRole('button', { name: /Start Terminal|Retry Terminal/ })).toBeEnabled();

  await page.getByRole('button', { name: 'Show Sidebar (⌘B)' }).click();
  await expect(page.getByRole('button', { name: 'Settings' })).toBeVisible();
  await page.getByRole('button', { name: 'Settings' }).click();

  const settings = page.getByRole('dialog', { name: 'Settings' });
  await expect(settings).toBeVisible();
  await expect
    .poll(async () => settings.evaluate(element => {
      const rect = element.getBoundingClientRect();
      return rect.left >= 0
        && rect.top >= 0
        && rect.right <= window.innerWidth
        && rect.bottom <= window.innerHeight;
    }))
    .toBe(true);

  await page.getByRole('tab', { name: 'Privacy' }).click();
  await expect(page.getByRole('heading', { name: 'Authenticated loopback inference' })).toBeVisible();
  await page.getByRole('button', { name: 'Close settings' }).click();
  await page.getByRole('button', { name: 'Close open side panel' }).click();

  await page.getByRole('button', { name: 'Show AI Panel (⌘J)' }).click();
  await expect(page.getByRole('heading', { name: 'Local Command Intelligence' })).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
    .toBe(true);
});
