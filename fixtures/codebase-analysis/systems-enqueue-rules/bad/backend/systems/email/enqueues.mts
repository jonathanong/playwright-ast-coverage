import { emailQueue } from './queues.mts';
import { trackJobEnqueue } from '../../../lib/tracking.mts';

export function enqueueMissingQueueAdd(payload: { to: string }) {
  trackJobEnqueue('email');
  return payload;
}

export function enqueueMissingTracking(payload: { to: string }) {
  return emailQueue.add('send', payload);
}

export function enqueueBulkWithoutBulk(items: { to: string }[]) {
  trackJobEnqueue('email', items, items.length);
  return emailQueue.add('send', items[0]);
}

export function sendEmail(payload: { to: string }) {
  trackJobEnqueue('email');
  return emailQueue.add('send', payload);
}
