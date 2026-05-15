test('external page', async ({ page }) => {
  await page.goto('https://example.com/users/42');
});
