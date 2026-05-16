import React from 'react'
import { cache } from 'react'

// bad: React.cache used outside web/lib/api/server/
export const userData = React.cache(async () => {
  return fetch('/api/user').then(r => r.json())
})

// bad: named cache import outside web/lib/api/server/
export const postData = cache(async () => {
  return fetch('/api/post').then(r => r.json())
})
