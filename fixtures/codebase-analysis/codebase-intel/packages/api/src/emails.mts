import { createQueue } from '@data-stores/valkey/glide-mq-factory';
export const emailsQueue = createQueue('emails', {});
