export async function getPostById(id: string) {
  return db.posts.findById(id)
}

export async function currentUserCanEditPost(ctx: Context, postId: string): Promise<void> {
  const post = await getPostById(postId)
  if (post.authorId !== ctx.user.id) {
    ctx.throw(403, 'forbidden')
  }
}
