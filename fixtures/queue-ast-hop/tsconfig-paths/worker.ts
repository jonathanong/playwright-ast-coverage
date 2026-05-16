import { Worker } from "bullmq";

export const worker = new Worker("email-paths", async (job) => {
  if (job.name === "pathJob") return job.data;
});
