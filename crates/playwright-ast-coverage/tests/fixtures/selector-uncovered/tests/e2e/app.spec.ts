import { test } from '@playwright/test';

test('covers route only', async ({ page }) => {
  await page.goto('/');
});
