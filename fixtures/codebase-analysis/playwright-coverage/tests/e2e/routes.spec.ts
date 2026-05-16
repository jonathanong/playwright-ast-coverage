test('covered routes', async ({ page }) => {
  await navigateTo(page, '/');
  await page.goto('/users/42');
});
