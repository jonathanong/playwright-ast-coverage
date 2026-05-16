import { createQueue } from '@data-stores/valkey/glide-mq-factory';
import { QUEUE_NAME } from './config.mts';

export const badWorkerQueue = createQueue(QUEUE_NAME);
