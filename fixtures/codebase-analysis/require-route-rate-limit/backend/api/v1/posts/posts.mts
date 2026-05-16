import app from '../../../app.mts';

// Missing ctx.applyRouteRateLimit() — should be flagged
app.route('/api/v1/posts').get(async (ctx) => {
  return ctx.json([]);
});

// Has rate limit — clean
app.route('/api/v1/posts').post(async (ctx) => {
  await ctx.applyRouteRateLimit('POST:/api/v1/posts');
  return ctx.json({ ok: true });
});
