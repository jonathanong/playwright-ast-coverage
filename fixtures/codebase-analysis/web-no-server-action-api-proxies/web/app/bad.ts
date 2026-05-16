import { serverApi } from '@/lib/api/server'

export async function submitForm(data: unknown) {
  "use server"
  return await serverApi.createRecord(data)
}
