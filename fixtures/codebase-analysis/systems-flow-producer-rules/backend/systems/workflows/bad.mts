import { autotaggerFlowProducer } from './queues.mts';

// Bad: calling flowProducer.add() outside enqueues.mts
export async function processAutotaggerFlow(job: any) {
  await autotaggerFlowProducer.add('autotagger', job.data);
}
