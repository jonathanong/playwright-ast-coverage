import { enqueueBulkEmails } from '../email/enqueues.mts';

for (const batch of batches) {
  await enqueueBulkEmails(batch);
}
