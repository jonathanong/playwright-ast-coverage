export async function loadUser() {
  return fetch('/api/v1/users/42');
}

export async function loadTopic(id: string) {
  return fetch(`/api/v1/topics/${id}`);
}

export function TopicLink({ id }: { id: string }) {
  return <a href={`/users/${id}`}>Topic</a>;
}
