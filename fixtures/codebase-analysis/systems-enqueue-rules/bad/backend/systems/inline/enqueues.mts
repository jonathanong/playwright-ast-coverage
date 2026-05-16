import { inlineQueue } from './queues.mts';
import { trackJobEnqueue } from '../../../lib/tracking.mts';
import { BULLMQ_INLINE_MODE } from './config.mts';

export function enqueueInlineMissingReturn(payload: { to: string }) {
  if (BULLMQ_INLINE_MODE) {
    inlineQueue.add('send', payload);
  }

  trackJobEnqueue('inline');
  inlineQueue.add('send', payload);
}

export function enqueueInlineReturnsOutsideInline(payload: { to: string }) {
  if (BULLMQ_INLINE_MODE) {
    return Promise.resolve({ inline: true });
  }

  trackJobEnqueue('inline');
  return inlineQueue.add('send', payload);
}
