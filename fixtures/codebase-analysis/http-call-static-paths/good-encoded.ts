export function replayDeadLetterJob(
  clientApi: { post(path: string): Promise<unknown> },
  queueName: string,
  jobId: string,
) {
  return clientApi.post(
    `/api/v1/mq/queues/${encodeURIComponent(queueName)}/dead-letter-jobs/${encodeURIComponent(jobId)}/replays`,
  );
}
