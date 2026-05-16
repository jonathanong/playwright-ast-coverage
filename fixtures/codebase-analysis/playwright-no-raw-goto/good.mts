import { expect, test } from '@playwright/test'

test('navigation test', async ({ page }) => {
  const response = await page.goto('/api/health')
  await page.goto('/sse', { waitUntil: 'load' })
})
