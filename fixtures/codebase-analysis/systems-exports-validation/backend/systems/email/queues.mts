// Bad: using new Queue() and string literal first arg
import { EMAIL_QUEUE_NAME } from './config.mts';
export const emailQueue = new Queue('email-bad-string');
