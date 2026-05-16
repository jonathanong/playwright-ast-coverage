import { createWorker } from '@data-stores/valkey/glide-mq-factory';

export const emailWorker = createWorker(emailQueue, processEmail, {
  lockDuration: 60_000,
});
