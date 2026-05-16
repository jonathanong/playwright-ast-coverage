import { emailQueue } from '../queues.mts';

// Bad: directly calling queue.add() outside an enqueues file
export async function processEmailBatch(job: any) {
  await emailQueue.add('email-send', job.data);
}
