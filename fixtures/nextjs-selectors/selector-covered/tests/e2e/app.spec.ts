import { test } from '@playwright/test';

test.describe('App', () => {
  test('covers selectors', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('save').click();
    await page.locator('[data-pw="publish"]').click();
    await (page.getByTestId)('save').click();
    await page.locator(`[data-testid="save"]`).click();
  });
});
