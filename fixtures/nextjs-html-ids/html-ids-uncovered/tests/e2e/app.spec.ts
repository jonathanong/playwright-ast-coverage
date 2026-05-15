import { test } from '@playwright/test';

test('misses one html id', async ({ page }) => {
  await page.goto('/');
  await page.locator('#save').click();
});
