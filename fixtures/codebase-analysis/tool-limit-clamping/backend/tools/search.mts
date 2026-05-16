export async function searchTool(limit: number) {
  const clamped = Math.min(limit, 100)
  return clamped
}
