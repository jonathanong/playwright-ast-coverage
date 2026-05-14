import { test } from '@playwright/test';

test('unsupported selector expression', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('anything').click();
});
