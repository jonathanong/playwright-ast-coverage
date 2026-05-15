export function getUsers() {
  return fetch('/api/users', { method: 'POST' });
}
