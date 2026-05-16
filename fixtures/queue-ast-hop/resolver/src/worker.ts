import * as worker from "@worker/worker";
import { Worker } from "bullmq";

new Worker("resolver", async (job) => {
  if (job.name === "run") return worker.handle(job);
});
