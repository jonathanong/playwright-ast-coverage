import { createQueue } from '@factory/glide-mq';
export const emails = createQueue('emails');
