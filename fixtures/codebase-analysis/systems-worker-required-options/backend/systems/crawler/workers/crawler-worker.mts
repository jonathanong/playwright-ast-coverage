import { Worker } from 'glide-mq';

export const crawlerWorker = new Worker(crawlerQueueName, processCrawlerJob, {
  lockDuration: 60_000,
  stalledInterval: 30_000,
});
