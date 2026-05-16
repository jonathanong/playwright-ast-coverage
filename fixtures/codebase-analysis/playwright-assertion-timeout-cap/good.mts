import { expect, test } from '@playwright/test'

test('example', async ({ page }) => {
  await expect(page.getByRole('heading')).toBeVisible({ timeout: 10_000 })
  await expect(page.getByRole('heading')).toBeVisible()
  await page.waitForURL('/dashboard', { timeout: 5_000 })
})
