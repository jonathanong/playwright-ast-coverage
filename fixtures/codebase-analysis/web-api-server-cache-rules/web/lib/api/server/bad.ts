import { cache } from 'react'

// bad: non-GET name inside web/lib/api/server/
export const sendMessage = cache(async () => {
  return fetch('/api/message').then(r => r.json())
})
