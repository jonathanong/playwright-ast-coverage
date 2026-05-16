import { emailQueue } from './queues.mts';

// Good: enqueues files are allowed to call queue.add() directly
export async function enqueueEmailSend(data: any) {
  await emailQueue.add('email-send', data);
}
