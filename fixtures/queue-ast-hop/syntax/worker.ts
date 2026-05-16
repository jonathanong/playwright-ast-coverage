import * as processor from "./processor";
import { Worker } from "bullmq";

new Worker("coverage", async (job) => {
  if (job.name === "welcome") return processor.handle(job);
});
