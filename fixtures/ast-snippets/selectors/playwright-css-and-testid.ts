await page.getByTestId('save').click();
await page.locator("[data-testid^='user-']").click();
await page.click('[data-pw$="button"]');
await page.locator('[data-pw*="nav"]');
await page.locator('[data-pw="exact"]');
await page.getByTestId(/^account-/);
