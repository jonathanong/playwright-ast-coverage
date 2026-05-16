import { expect, test } from '@playwright/test'

test('example', async ({ page }) => {
  await expect(page.getByRole('heading')).toBeVisible({ timeout: 15_000 })
  await page.waitForURL('/dashboard', { timeout: 15_000 })
})
