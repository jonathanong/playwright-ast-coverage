import { unstable_cache, revalidatePath } from 'next/cache'

export const revalidate = 60
export const fetchCache = 'force-cache'

export const cachedUser = unstable_cache(async () => {
  return fetch('/api/user').then(r => r.json())
})
