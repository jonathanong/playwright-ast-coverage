import { use } from 'react'

const user = use(Promise.resolve(fetchUser()))
const settings = use(Promise.resolve({ theme: 'dark' }))
