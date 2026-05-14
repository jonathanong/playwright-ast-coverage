import { test } from '@playwright/test';

test('covers html ids', async ({ page }) => {
  await page.goto('/');
  await page.locator('#save').click();
  await page.locator('[id="publish"]').click();
});
