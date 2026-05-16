import { createQueue } from '@data-stores/valkey/glide-mq-factory';
import { DEAD_LETTER_QUEUE_NAME, QUEUE_NAME } from './config.mts';

export const createWorkerGood = createQueue(QUEUE_NAME, {
  deadLetterQueue: { name: DEAD_LETTER_QUEUE_NAME },
});
