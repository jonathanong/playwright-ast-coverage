export function processLocalData() {
  const data = [1, 2, 3, 4, 5]
  return data.find(x => x > 3)
}

export function sortLocal() {
  const items = ['c', 'a', 'b']
  return items.sort()
}

async function fetchUsers() {
  return Promise.resolve([{ id: 123, name: 'Alice' }])
}

export async function useAllowedMethod() {
  const users = await fetchUsers()
  users.forEach(u => console.log(u.name))
}
