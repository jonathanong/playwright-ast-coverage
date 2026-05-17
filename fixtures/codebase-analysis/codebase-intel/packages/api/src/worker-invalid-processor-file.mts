import * as processors from './processors-invalid.mts';

new Worker('emails', async (job) => processors[job.name]());
