import { Queue, Worker } from "glide-mq";

const QUEUE_NAME = "events";
export const eventsQueue = new Queue(QUEUE_NAME);

export function enqueueDynamic(jobName: string) {
  return eventsQueue.add(jobName, {});
}

export const worker = new Worker(QUEUE_NAME, async (job) => {
  if (job.name === "staticJob") {
    return job.data;
  }
});
