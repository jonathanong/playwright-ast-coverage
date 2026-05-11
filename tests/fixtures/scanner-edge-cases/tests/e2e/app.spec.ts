import { expect, test } from '@playwright/test';

test('covers scanner edge cases', async ({ page }) => {
  const sample = "escaped \".goto('/string-example')";
  await page.goto(   '/');
  // await page.goto('/line-comment');
  /* await page.click('a[href="/block-comment"]'); */
  await page.goto('/docs/guides/install');
  await page.goto('/shop');
  await page.goto('http://localhost:3000/shop/shoes/red');
  await page.goto('//example.com/external');
  await page.goto(buildUrl('/dynamic'));
  await page.click(selectorFor('a[href="/dynamic-click"]'));
  await page.click('a[href="/settings"]');
  await expect(page).toHaveURL('settings', '/settings');
  await page.getByTestId('save').click();
  await page.locator('[data-pw="publish"]').click();
  // await page.getByTestId('commented-line').click();
  /* await page.locator('[data-testid="commented-block"]').click(); */
});
