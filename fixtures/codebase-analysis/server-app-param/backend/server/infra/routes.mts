import { Application } from '@modules/api-server';

// This violates the rule — must register routes inline, not via parameterized function
function registerHealthRoutes(app: Application): void {
  app.route('/api/v1/health').get(async () => new Response('ok'));
}
