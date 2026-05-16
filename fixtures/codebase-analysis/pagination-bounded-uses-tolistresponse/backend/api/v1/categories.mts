import { toListResponse } from '@modules/pagination'

router.get('/api/v1/categories', async (ctx) => {
  const categories = await db.categories.findMany()
  ctx.body = toListResponse(categories)
})
