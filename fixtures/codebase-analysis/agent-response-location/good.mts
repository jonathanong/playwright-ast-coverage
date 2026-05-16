// This file is NOT in backend/ — should not be flagged
import { createOpenAIResponse } from '@modules/openai-utils/client.mts'

export async function utility() {
  return createOpenAIResponse({ model: 'gpt-4o', input: 'hello', safety_identifier: 'utility' })
}
