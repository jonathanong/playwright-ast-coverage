import { emailsQueue } from "./queues";

export function enqueueWelcome(userId: string) {
  return emailsQueue.add("sendWelcome", { userId });
}
