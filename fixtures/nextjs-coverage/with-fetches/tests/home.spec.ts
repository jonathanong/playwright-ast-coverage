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

test('clicks relative href', async ({ page }) => {
  await page.goto('/');
  await page.click('a[href="relative-path"]');
});

test('visits with template literal', async ({ page }) => {
  const path = 'home';
  await page.goto(`/${path}`);
  await expect(page).toHaveURL(`/${path}`);
});

test('visits with identifier', async ({ page }) => {
  const dest = '/';
  await page.goto(dest);
});
