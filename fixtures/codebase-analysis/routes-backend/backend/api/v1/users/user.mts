import app from '../../app.mts'
import type { Context } from '@modules/api-server'

app
  .route('/api/v1/users/:idOrSlug')
  .get(async (ctx: Context) => {
    const idOrSlug = ctx.params.idOrSlug!
    ctx.assert(idOrSlug, 400, 'idOrSlug is required')
    ctx.body = { id: idOrSlug }
  })
  .patch(async (ctx: Context) => {
    const idOrSlug = ctx.params.idOrSlug!
    ctx.body = { id: idOrSlug, updated: true }
  })
  .delete(async (ctx: Context) => {
    const idOrSlug = ctx.params.idOrSlug!
    ctx.status = 204
  })
