import { test } from '@playwright/test';

test('custom playwright test id attribute', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('publish').click();
});
