import { createFlowProducer } from 'glide-mq-factory';

// Good: new FlowProducer allowed here
export const autotaggerFlowProducer = new FlowProducer({ connection });
export const transcriptFlowProducer = createFlowProducer({ connection });
