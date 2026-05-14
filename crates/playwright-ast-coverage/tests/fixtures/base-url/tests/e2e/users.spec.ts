test('user page', async ({ page }) => {
  await page.goto('http://localhost:3000/users/42');
});
