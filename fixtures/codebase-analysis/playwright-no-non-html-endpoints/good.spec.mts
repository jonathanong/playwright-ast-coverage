import { test, expect } from '@playwright/test'

test('only navigates HTML pages', async ({ page, request }) => {
  await page.goto('/about')
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
  await request.get('/about')
  await request.get('/contact')
})
