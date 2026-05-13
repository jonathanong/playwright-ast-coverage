import { test } from '@playwright/test';

test('covers component selector attributes', async ({ page }) => {
  await page.goto('/');
  await page.locator('[data-pw="save"]').click();
});
