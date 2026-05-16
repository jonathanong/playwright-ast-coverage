test('user page', async ({ page }) => {
  await page.goto('/users/42');
  const id = '42';
  await page.goto(`/users/${id}`);
});
