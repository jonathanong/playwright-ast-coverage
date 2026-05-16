import app from '../../../app.mts';

// RPC-style verb in path — should be flagged
app.route('/api/v1/posts/:id/share').post(async (ctx) => {
  return ctx.json({ ok: true });
});

// Another RPC verb
app.route('/api/v1/posts/:id/publish').post(async (ctx) => {
  return ctx.json({ ok: true });
});

// Noun-based path — clean
app.route('/api/v1/posts/:id/comments').get(async (ctx) => {
  return ctx.json([]);
});
