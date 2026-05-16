// Good: importing own system's internal files is allowed
import type { EmailJob } from '@systems/email/types';
import type { EmailConfig } from '@systems/email/types.mts';

export function processEmail(job: EmailJob) {
  return job;
}
