import { createQueue } from '@factory/glide-mq';
export const payments = createQueue('payments');
