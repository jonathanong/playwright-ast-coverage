test('does not cover empty dynamic segment', async ({ page }) => {
  await page.goto('/users//settings');
});
