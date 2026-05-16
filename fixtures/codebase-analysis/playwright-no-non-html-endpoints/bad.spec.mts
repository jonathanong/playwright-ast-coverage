import { test } from '@playwright/test'

test('non-HTML endpoints', async ({ page, request }) => {
  await request.get('/api/v1/users')
  await request.post('/api/v1/users', { data: {} })
  await page.request.get('/rss/feed.xml')
  await request.get('/infra/health')
  await request.get('/robots.txt')
  await request.get('/llms.txt')
  await request.get('/sitemap.xml')
  await request.get('/md/about')
})
