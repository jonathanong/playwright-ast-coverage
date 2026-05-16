import { createQueue } from 'glide-mq-factory';

export const emailQueue = createQueue(EMAIL_QUEUE_NAME);
