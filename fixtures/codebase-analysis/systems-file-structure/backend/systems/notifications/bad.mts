// Bad: importing internal files from another system
import type { EmailJob } from '@systems/email/types';
import type { EmailConfig } from '@systems/email/processors.mts';

export function handleNotification() {}
