import { test } from '@playwright/test';

test('visits home', async ({ page }) => {
  await page.goto('/');
});
