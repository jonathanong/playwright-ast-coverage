import { test } from '@playwright/test';

test('view user', async ({ page }) => {
    await page.goto('/users/42');
});
