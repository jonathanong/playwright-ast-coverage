export function loadUser(clientApi: { get(path: string): Promise<unknown> }, userId: string) {
  return clientApi.get(`/api/v1/users/${userId}`);
}
