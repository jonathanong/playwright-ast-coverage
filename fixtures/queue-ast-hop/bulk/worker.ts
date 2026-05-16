import { Worker } from "glide-mq";
import * as processors from "./processors";

export const worker = new Worker("bulk", async (job) => processors[job.name](job.data));
