import { test, expect } from '@playwright/test';

test('user story', async ({ page }) => {
  await page.goto('/users/42');
  await expect(page.getByTestId('user-page')).toBeVisible();
});
