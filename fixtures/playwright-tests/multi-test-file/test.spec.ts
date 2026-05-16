import { test } from '@playwright/test';

test.describe('Users', () => {
  test('lists users', async ({ page }) => {
    await page.goto('/users');
    await page.getByTestId('user-list').click();
  });

  test('views user', async ({ page }) => {
    await page.goto('/users/1');
    await page.getByTestId('user-profile').click();
  });
});

test('home page', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('hero').click();
});
