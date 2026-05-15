import { test } from '@playwright/test';

test('selectors disabled', async ({ page }) => {
  await page.goto('/');
});
