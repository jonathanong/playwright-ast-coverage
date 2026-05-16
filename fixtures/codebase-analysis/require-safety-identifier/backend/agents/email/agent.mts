// Missing safety_identifier — should be flagged
export async function runEmailAgent() {
  const result = await createOpenAIResponse({ model: 'gpt-4o', input: 'draft email' })
  return result
}
