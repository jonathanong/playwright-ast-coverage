import { test } from '@playwright/test';

test('covers selectors', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('save').click();
  await page.locator('[data-pw="publish"]').click();
});
