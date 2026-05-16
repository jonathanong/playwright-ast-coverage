import {
  createBulkEnqueueFunction,
  createEnqueueFunction,
} from '@data-stores/valkey/glide-mq-factory';
import * as glideMqFactory from '@data-stores/valkey/glide-mq-factory';
import { tryFactoryQueue } from './queues.mts';

function createCatchFactory(jobName: string) {
  try {
    throw new Error('force catch branch');
  } catch {
    return createEnqueueFunction({
      queue: tryFactoryQueue,
      queueName: 'try-factory',
      jobName,
    });
  }
}

function createFinallyFactory(jobName: string) {
  try {
    return createBulkEnqueueFunction({
      queue: tryFactoryQueue,
      queueName: 'try-factory',
      jobName,
      buildJob: id => ({ data: { id } }),
    });
  } finally {
    cleanupFactorySetup();
  }
}

const createNamespaceFactory = () =>
  glideMqFactory.createEnqueueFunction({
    queue: tryFactoryQueue,
    queueName: 'try-factory',
    jobName: 'namespaceFactory',
  });

const catchFactoryJob = createCatchFactory('catchFactory');
const bulkFinallyFactoryJobs = createFinallyFactory('finallyFactory');
const namespaceFactoryJob = createNamespaceFactory();

export function enqueueCatchFactory(id: string) {
  return catchFactoryJob({ id });
}

export function enqueueBulkFinallyFactory(ids: string[]) {
  return bulkFinallyFactoryJobs(ids);
}

export function enqueueNamespaceFactory(id: string) {
  return namespaceFactoryJob({ id });
}
