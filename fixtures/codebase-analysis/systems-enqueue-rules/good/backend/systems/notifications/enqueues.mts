import {
  createBulkEnqueueFunction,
  createEnqueueFunction as createGlideMqEnqueueFunction,
} from '@data-stores/valkey/glide-mq-factory';
import { notifications } from './queues.mts';

const enqueueReferralSignupNotificationJob = createGlideMqEnqueueFunction({
  queue: notifications,
  queueName: 'notifications',
  jobName: 'processReferralSignupNotification',
});

const enqueueBulkFollowNotificationJobs = createBulkEnqueueFunction({
  queue: notifications,
  queueName: 'notifications',
  jobName: 'processFollowNotification',
  buildJob: data => ({
    data,
    opts: {
      deduplication: {
        id: `processFollowNotification__${data.followeeId}__${data.followerId}`,
        mode: 'debounce',
        ttl: 60_000,
      },
    },
  }),
});

export function enqueueReferralSignupNotification(referrerId: string, newUserId: string) {
  return enqueueReferralSignupNotificationJob(
    { referrerId, newUserId },
    {
      deduplication: {
        id: `processReferralSignupNotification__${referrerId}__${newUserId}`,
        mode: 'debounce',
        ttl: 60_000,
      },
    },
  );
}

export function enqueueBulkFollowNotification(
  pairs: Array<{ followeeId: string; followerId: string }>,
) {
  return enqueueBulkFollowNotificationJobs(pairs);
}
