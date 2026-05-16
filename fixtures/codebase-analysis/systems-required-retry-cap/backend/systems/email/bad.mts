import { emailQueue } from './queues.mts';

// Bad: missing attempts
export async function enqueueEmail(data: any) {
  await emailQueue.add('email', data, { timeout: 5000 });
}
