import { test } from '@playwright/test';

test('covers fuzzy selectors', async ({ page }) => {
  await page.goto('/users/42');
  await page.getByTestId('user-42').click();
  await page.locator('[data-pw="user-42-link"]');
  await page.locator('[data-pw^="user-"]').click();
});
