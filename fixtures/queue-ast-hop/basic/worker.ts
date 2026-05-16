import { Worker } from "bullmq";

export const worker = new Worker("emails", async (job) => {
  if (job.name === "sendWelcome") {
    return processWelcome(job.data);
  }
});

function processWelcome(data: unknown) {
  return data;
}
