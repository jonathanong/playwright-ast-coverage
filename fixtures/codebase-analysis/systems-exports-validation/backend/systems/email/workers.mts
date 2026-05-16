import { Worker } from 'bullmq';
export const emailWorker = new Worker('email', processor, {});
