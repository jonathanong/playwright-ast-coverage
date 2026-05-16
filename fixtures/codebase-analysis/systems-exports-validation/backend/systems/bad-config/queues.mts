import { createQueue } from '@data-stores/valkey/glide-mq-factory';
import { QUEUE_NAME } from './config.mts';

export const badConfigQueue = createQueue(QUEUE_NAME);
