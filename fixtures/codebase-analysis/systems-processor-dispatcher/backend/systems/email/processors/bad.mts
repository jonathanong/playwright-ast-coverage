import { enqueueBulkEmailsSend } from '../../enqueues.mts';

// Bad: calls enqueueBulk but name doesn't contain "Dispatcher"
export async function processEmailBatch(job: any) {
  await enqueueBulkEmailsSend([job.data]);
}
