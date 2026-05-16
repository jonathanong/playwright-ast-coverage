export async function runAgent() {
  const result1 = await runToolLoop({ model: 'gpt-4o', tools: [], input: 'first' })
  return result1
}
