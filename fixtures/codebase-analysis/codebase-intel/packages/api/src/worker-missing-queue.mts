import * as processors from './processors.mts';

new Worker('missing-queue', async (job) => processors[job.name]());
