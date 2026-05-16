import { enqueueEmail } from '../email/enqueues.mts';

for (const item of items) {
  await enqueueEmail(item);
}

await Promise.all([enqueueEmail(a), enqueueEmail(b)]);
