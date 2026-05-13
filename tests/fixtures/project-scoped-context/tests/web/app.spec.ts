import { expect, test } from '@playwright/test';

test('web project context only', async ({ page }) => {
  await page.goto('http://localhost:3000/');
  await page.goto('http://localhost:6006/admin');
  await page.getByTestId('home').click();
  await page.getByTestId('publish').click();
  await expect(page).not.toHaveURL('/admin');
});
