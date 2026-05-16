import unusedDefault from "./unused-default";
import { bulkQueue } from "./queues";

void unusedDefault;

export function enqueueBulk() {
  return bulkQueue.addBulk([
    { name: "jobA", data: {} },
    { name: "jobB", data: {} },
  ]);
}
