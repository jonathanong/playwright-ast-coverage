import * as processors from './missing-processors.mts';

new Worker('emails', async (job) => processors[job.name]());
