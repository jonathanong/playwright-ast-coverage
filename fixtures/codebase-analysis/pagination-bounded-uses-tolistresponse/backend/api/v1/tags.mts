router.get('/api/v1/tags', async (ctx) => {
  const tags = await db.tags.findMany()
  ctx.body = { results: tags, count: tags.length }
})
