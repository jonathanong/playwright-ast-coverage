import { Queue, Worker } from "bullmq";

export const lonelyQueue = new Queue("lonely");

export function enqueueLonely() {
  return lonelyQueue.add("missingWorker", {});
}

export const worker = new Worker("lonely", async (job) => {
  if (job.name === "missingProducer") return job.data;
});
