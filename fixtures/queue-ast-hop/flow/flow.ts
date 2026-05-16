import { FlowProducer, Worker } from "bullmq";

const flow = new FlowProducer();
const JOB = "resize";

export function enqueueFlow() {
  return flow.add({ name: JOB, queueName: "images", data: {} });
}

export const worker = new Worker("images", async (job) => {
  if (job.name === "resize") {
    return job.data;
  }
});
