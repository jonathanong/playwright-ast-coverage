async function fetchUsers() {
  return Promise.resolve([{ id: 123, name: 'Alice' }])
}

export async function badFind() {
  const users = await fetchUsers()
  return users.find(u => u.id === 123)
}

export async function badSort() {
  const rows = await fetchUsers()
  return rows.sort((a, b) => a.id - b.id)
}

export async function badReduce() {
  const result = await fetchUsers()
  return result.reduce((acc: number, x) => acc + x.id, 0)
}
