import { use } from 'react'

// good: use() with a real promise, not Promise.resolve
const userPromise = fetchUser()
const user = use(userPromise)

// good: Promise.resolve without use()
const val = Promise.resolve(42)
