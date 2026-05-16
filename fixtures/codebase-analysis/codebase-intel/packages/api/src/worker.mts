import * as processors from './processors.mts';

new Worker('emails', async (job) => processors[job.name]());
