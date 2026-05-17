import defaultQueue from "./default-queue.mts";
import { emailsQueue } from "./queues.mts";
import * as processors from "./processors.mts";
import "./side-effect.mts";

declare function ambientQueue(): void;
emailsQueue.add("top", {});
defaultQueue.add(jobName, {});

export function exportedRunner() {
  return emailsQueue.add("returned", {});
}

export class IgnoredExport {}

{
  emailsQueue.add("block", {});
}

if (ready) {
  emailsQueue.add("if", {});
} else {
  emailsQueue.add("else", {});
}

try {
  emailsQueue.add("try", {});
} catch (error) {
  emailsQueue.add("catch", {});
}

function localRunner() {
  emailsQueue.add("function", {});
}

export const nested = () => {
  emailsQueue.add("arrow", {});
  emailsQueue.addBulk([{ name: "bulk" }, { name: jobName }, spreadItem]);
  Promise.resolve(emailsQueue.add("nested-arg", {}));
  const casted = emailsQueue.add("casted", {}) as unknown;
  const nonNull = emailsQueue.add("nonnull", {})!;
  emailsQueue?.add;
};

export async function awaitedRunner() {
  await emailsQueue.add("awaited", {});
}

export const worker = new Worker("emails", (job) => processors[job.name](job.data));
export const dynamicWorker = new Worker(QUEUE_NAME, handler);
export const notWorker = new SomethingElse("emails");
maybe?.queue;
