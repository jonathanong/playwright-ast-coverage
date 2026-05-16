import { Queue, FlowProducer } from 'glide-mq'

const workerQueueConnection = { host: 'localhost', port: 6379 }
const workerQueuePrefix = '{bull}'

export function createQueue<T = unknown>(name: string): Queue<T> {
  return new Queue<T>(name, {
    connection: workerQueueConnection,
    prefix: workerQueuePrefix,
  })
}

export function createFlowProducer(): FlowProducer {
  return new FlowProducer({
    connection: workerQueueConnection,
    prefix: workerQueuePrefix,
  })
}
