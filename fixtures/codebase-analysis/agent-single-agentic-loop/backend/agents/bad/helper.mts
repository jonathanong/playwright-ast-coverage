export async function helperLoop() {
  const result2 = await createOpenAIResponse({ model: 'gpt-4o', input: 'second', safety_identifier: 'helper' })
  return result2
}
