import { test } from '@playwright/test';

test('home', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('save').click();
});
