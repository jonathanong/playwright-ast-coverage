test('user page', async ({ page }) => {
  await page.goto('/users/42');
});
