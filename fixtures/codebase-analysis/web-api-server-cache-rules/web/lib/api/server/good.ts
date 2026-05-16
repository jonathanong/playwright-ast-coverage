import { cache } from 'react'

// good: GET-prefixed cache inside web/lib/api/server/
export const getUser = cache(async () => {
  return fetch('/api/user').then(r => r.json())
})
