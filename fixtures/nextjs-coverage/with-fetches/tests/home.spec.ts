test.describe('Home', () => {
  test('visits home page', async ({ page }) => {
    await page.goto('/');
  });
});

test('also visits', async ({ page }) => {
  await page.goto('/');
});

const dynamicName = 'dynamic test';
test(dynamicName, async ({ page }) => {
  await page.goto('/');
});
