export function loadTopics(clientApi: { get(path: string): Promise<unknown> }) {
  return clientApi.get('/api/v1/topics');
}
