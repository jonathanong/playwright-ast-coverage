import { test } from '@playwright/test';

test('covers route and component selectors', async ({ page }) => {
  await page.goto('/');
  await page.locator('[data-pw="save-button"]').click();
});
