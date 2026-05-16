import { Worker } from 'glide-mq';
import { QUEUE_NAME } from './config.mts';

export const badQueueWorker = new Worker(QUEUE_NAME, async job => job.data, {});
