import { getUser } from '@/lib/api/client'

export async function loadPage() {
  return getUser()
}
