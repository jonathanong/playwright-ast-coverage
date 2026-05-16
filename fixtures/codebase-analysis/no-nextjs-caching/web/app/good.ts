export const dynamic = 'force-dynamic'

export async function load() {
  return fetch('/api/user').then(r => r.json())
}
