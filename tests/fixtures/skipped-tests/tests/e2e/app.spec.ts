import { test, expect } from '@playwright/test';

test.skip('skipped route and selector', async ({ page }) => {
  await page.goto('/skipped');
  await expect(page.getByTestId('skipped-page')).toBeVisible();
});

test('conditionally skipped route and selector', async ({ page, browserName }) => {
  test.skip(browserName === 'webkit', 'covered conditionally');
  await page.goto('/conditional');
  await expect(page.getByTestId('conditional-page')).toBeVisible();
});

test('mixed active coverage wins', async ({ page }) => {
  await page.goto('/mixed');
  await expect(page.getByTestId('mixed-page')).toBeVisible();
});

