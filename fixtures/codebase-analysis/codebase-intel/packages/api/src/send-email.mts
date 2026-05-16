import { emailsQueue } from './emails.mts';

export function enqueueWelcomeEmail() {
  return emailsQueue.add('sendWelcomeEmail', { userId: 'user-1' });
}
