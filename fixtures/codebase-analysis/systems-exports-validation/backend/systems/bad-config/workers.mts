import { createWorker } from '@data-stores/valkey/glide-mq-factory';
import { QUEUE_NAME } from './config.mts';

export const badConfigWorker = createWorker(QUEUE_NAME, async job => job.data, {});
