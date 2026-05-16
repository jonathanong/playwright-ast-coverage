import { inlineQueue } from './queues.mts';
import { trackJobEnqueue } from '../../../lib/tracking.mts';
import { BULLMQ_INLINE_MODE } from './config.mts';

export function enqueueInlineEmail(payload: { to: string }) {
  if (BULLMQ_INLINE_MODE) {
    return Promise.resolve({ inline: true });
  }

  trackJobEnqueue('inline');
  inlineQueue.add('send', payload);
}
