import { test } from '@playwright/test'

test('low-priority selectors', async ({ page }) => {
  await page.locator('text=Submit').click()
  await page.locator('button:has-text("Save")').click()
  await expect(page.locator('h1')).toBeVisible()
  await page.locator('input[placeholder*="Email"]').fill('a@b.com')
  await page.click('.legacy-btn')
  await page.fill('#email', 'a@b.com')
  await page.waitForSelector('text=Ready')
  await page.waitForSelector('div:has-text("Loaded")')
})
