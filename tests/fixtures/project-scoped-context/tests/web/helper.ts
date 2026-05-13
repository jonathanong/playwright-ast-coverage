import { test } from '@playwright/test';

test('does not match project testMatch', async ({ page }) => {
  await page.goto('http://localhost:6006/admin');
  await page.getByTestId('home').click();
});
