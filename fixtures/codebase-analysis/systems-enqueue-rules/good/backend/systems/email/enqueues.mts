import { emailQueue } from './queues.mts';
import { trackJobEnqueue } from '../../../lib/tracking.mts';

export function enqueueEmail(payload: { to: string }) {
  trackJobEnqueue('email');
  return emailQueue.add('send', payload);
}

export const enqueueBulkEmails = (items: { to: string }[]) => {
  trackJobEnqueue('email', items, items.length);
  return emailQueue.addBulk(items.map(p => ({ name: 'send', data: p })));
};
