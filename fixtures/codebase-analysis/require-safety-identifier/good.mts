// Not in backend/agents/ — should not be flagged
export async function utility() {
  return createOpenAIResponse({ model: 'gpt-4o', input: 'hi' })
}
