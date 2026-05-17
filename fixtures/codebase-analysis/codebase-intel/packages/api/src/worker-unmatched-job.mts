import * as processors from './processors-extra.mts';

new Worker('emails', async (job) => processors[job.name]());
