export async function runGoodAgent() {
  const result = await runToolLoop({ model: 'gpt-4o', tools: [], input: 'run' })
  return result
}
