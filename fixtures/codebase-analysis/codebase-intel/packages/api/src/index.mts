import app from './app.mts';
import { emailsQueue } from './emails.mts';
export { emailsQueue };
app.route('/api/v1/users/:id').get((_req, res) => res.json({ id: 1 }));
app.get('/api/v1/topics/:id', (_req, res) => res.json({ id: 1 }));
