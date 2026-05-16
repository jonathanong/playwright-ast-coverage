import { test } from '@playwright/test'

test('navigation test', async ({ page }) => {
  await page.goto('/dashboard')
  await page.goto('/', { waitUntil: 'domcontentloaded' })
})
