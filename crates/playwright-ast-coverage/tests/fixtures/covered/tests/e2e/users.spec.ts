test('user page', async ({ page }) => {
  await page.goto('about:blank');
  await page.goto('/users/42');
  await page.goto("/users/42");
  await page.click('button.submit');
  await page.click('a[href="/users/42"]');
  await page.click(`a[href='/users/42']`);
  await expect(page).toHaveURL('user detail', '/users/42');
});
