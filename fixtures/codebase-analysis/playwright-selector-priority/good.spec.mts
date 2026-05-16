import { expect, test } from '@playwright/test'

test('semantic selectors', async ({ page }) => {
  await page.getByRole('button', { name: 'Submit' }).click()
  await page.getByRole('button', { name: 'Save' }).click()
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
  await page.getByPlaceholder('Email').fill('a@b.com')
  await page.locator('[data-testid="legacy-btn"]').click()
  await page.locator('script[type="application/ld+json"]').textContent()
  await page.locator('button').first().click()
})
