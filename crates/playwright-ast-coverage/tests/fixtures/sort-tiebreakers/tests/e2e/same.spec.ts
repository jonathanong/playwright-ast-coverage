import { test } from '@playwright/test';

test('covers duplicates', async ({ page }) => {
  await page.goto('/same');
  await page.getByTestId('dup').click();
});
