import { emailQueue } from "@queues/email";

export function enqueuePathJob() {
  return emailQueue.add("pathJob", {});
}
