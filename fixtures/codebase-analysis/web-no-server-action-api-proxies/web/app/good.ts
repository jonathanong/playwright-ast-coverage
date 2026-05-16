import { clientApi } from '@/lib/api/client'

export async function submitForm(data: unknown) {
  "use server"
  // Calling clientApi is fine — only serverApi is banned in server actions
  return { ok: true }
}

export async function normalFn() {
  // Not a server action — calling serverApi here is fine
  return true
}
