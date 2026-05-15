import { test } from '@playwright/test';

test('ids are ignored by default', async ({ page }) => {
  await page.goto('/');
});
