import { getPostById } from './posts.mts'

export async function currentUserCanReadPost(ctx: Context, postId: string): Promise<void> {
  const post = await getPostById(postId)
  if (!post) {
    ctx.throw(404, 'post not found')
  }
}
