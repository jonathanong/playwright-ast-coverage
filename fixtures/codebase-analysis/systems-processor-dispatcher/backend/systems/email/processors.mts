import { enqueueBulkEmailsSend } from '../enqueues.mts';

// Good: calls enqueueBulk and has "Dispatcher" in name
export async function processEmailDispatcher(job: any) {
  await enqueueBulkEmailsSend([job.data]);
}

// Good: does NOT call enqueueBulk — no Dispatcher required
export async function processWelcomeEmail(job: any) {
  await Promise.resolve(job.data);
}
