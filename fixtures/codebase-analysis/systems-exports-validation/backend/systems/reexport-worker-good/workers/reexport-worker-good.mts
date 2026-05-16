import { Worker } from 'glide-mq';
import { workerQueueConnection, workerQueuePrefix } from '@data-stores/valkey/glide-mq-client';
import { QUEUE_NAME } from '../config.mts';

export const reexportWorkerGood = new Worker(
  QUEUE_NAME,
  async job => {
    return job.data;
  },
  {
    connection: workerQueueConnection,
    prefix: workerQueuePrefix,
  },
);
