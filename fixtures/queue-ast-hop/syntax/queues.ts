import { FlowProducer, Queue, TestQueue, Worker } from "bullmq";

const QUEUE = "coverage";
const JOB = "welcome";
const DYNAMIC_JOB = String("dynamic");
const { ignored } = process.env;

export const queue = new Queue(QUEUE);
export const testQueue = new TestQueue("coverage-test");
const unresolvedQueue = new Queue(getQueueName());

queue.add(JOB, {});
queue.add('say"hi', {});
queue.addBulk([{ name: JOB }, { name: DYNAMIC_JOB }, { id: 1 }, { ...process.env }, "skip"]);
queue.addBulk();
unresolvedQueue.add("lost", {});
getQueue().add("lost", {});

const flow = new FlowProducer();
flow.add({ name: JOB });
flow.add("skip");
flow.add({ ...process.env, name: JOB, queueName: QUEUE });

new Worker(QUEUE, async (job) => {
  if (job.name === "welcome") return import("./processor");
  if (job.name === "say\"hi") return null;
});

new Worker(QUEUE, { name: "welcome" });
new Worker(QUEUE, { name: getQueueName() });
new Worker(getQueueName(), async () => {});

function getQueueName() {
  return ignored ?? "coverage";
}

function getQueue() {
  return queue;
}
