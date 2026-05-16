import { createWorker } from '@data-stores/valkey/glide-mq-factory';
import { DEAD_LETTER_QUEUE_NAME, QUEUE_NAME } from './config.mts';

export const createWorkerGood = createWorker(
  QUEUE_NAME,
  async job => {
    return job.data;
  },
  {
    concurrency: 10,
    deadLetterQueue: { name: DEAD_LETTER_QUEUE_NAME },
  },
);
