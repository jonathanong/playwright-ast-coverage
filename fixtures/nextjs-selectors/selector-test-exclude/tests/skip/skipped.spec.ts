import { test } from '@playwright/test';

// This file is intentionally excluded via testExclude in the yaml config.
test('this test is excluded', async ({ page }) => {
  await page.goto('/');
});
