import { test } from '@playwright/test';

test('covers custom attrs', async ({ page }) => {
  await page.goto('/');
  await page.locator('[data-test*="save"]').click();
  await page.getByTestId('card-1').click();
});
