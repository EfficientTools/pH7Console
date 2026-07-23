import { test, expect } from '@playwright/test';

test.describe('pH7Console desktop shell', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'pH7Console' })).toBeVisible();
  });

  test('renders the primary desktop regions', async ({ page }) => {
    await expect(page.getByRole('tab', { name: 'Terminals' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Explorer' })).toBeVisible();
    await expect(page.getByText('No active terminal session')).toBeVisible();
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
    await expect(page.getByText('Local AI is unavailable')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Retry AI' })).toBeVisible();
    await expect(page.getByText('AI Ready')).toHaveCount(0);
  });
});
