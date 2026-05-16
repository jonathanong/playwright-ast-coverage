import type { Context } from '@app/context';

export async function getPosts(ctx: Context) {
  // accessing .roles outside of services — should be flagged
  if (ctx.user.roles.includes('admin')) {
    return ctx.db.posts.findAll();
  }
  return ctx.db.posts.findPublic();
}
