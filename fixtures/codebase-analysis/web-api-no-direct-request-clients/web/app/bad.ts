import { clientApi } from '@/lib/api/client'
import { serverApi } from '@/lib/api/server'

export async function loadPage() {
  const user = await clientApi.getUser()
  const settings = await serverApi.getSettings()
  return { user, settings }
}
