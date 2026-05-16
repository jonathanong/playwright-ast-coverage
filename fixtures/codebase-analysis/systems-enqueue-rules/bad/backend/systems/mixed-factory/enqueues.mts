import {
  createBulkEnqueueFunction,
  createEnqueueFunction,
} from '@data-stores/valkey/glide-mq-factory';
import { mixedFactoryQueue } from './queues.mts';

const singleJob = createEnqueueFunction({
  queue: mixedFactoryQueue,
  queueName: 'mixed-factory',
  jobName: 'single',
});

const bulkJobs = createBulkEnqueueFunction({
  queue: mixedFactoryQueue,
  queueName: 'mixed-factory',
  jobName: 'bulk',
  buildJob: id => ({ data: { id } }),
});

function createMixedFactory(useBulk: boolean) {
  if (useBulk) {
    return createBulkEnqueueFunction({
      queue: mixedFactoryQueue,
      queueName: 'mixed-factory',
      jobName: 'mixedBulkCreator',
      buildJob: id => ({ data: { id } }),
    });
  }

  return createEnqueueFunction({
    queue: mixedFactoryQueue,
    queueName: 'mixed-factory',
    jobName: 'mixedSingleCreator',
  });
}

const mixedCreatorJob = createMixedFactory(process.env.USE_BULK === '1');

export function enqueueBulkMixedFactory(ids: string[], useBulk: boolean) {
  if (useBulk) {
    return bulkJobs(ids);
  }

  return singleJob({ id: ids[0] });
}

export function enqueueMixedFactory(ids: string[], useBulk: boolean) {
  if (useBulk) {
    return bulkJobs(ids);
  }

  return singleJob({ id: ids[0] });
}

export function enqueueBulkMixedCreator(ids: string[]) {
  return mixedCreatorJob(ids);
}

export function enqueueMixedCreator(ids: string[]) {
  return mixedCreatorJob(ids);
}
