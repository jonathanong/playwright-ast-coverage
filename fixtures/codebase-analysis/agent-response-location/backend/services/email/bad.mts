import { createOpenAIResponse } from '@modules/openai-utils/client.mts'

export async function sendEmail() {
  return createOpenAIResponse({ model: 'gpt-4o', input: 'draft email', safety_identifier: 'send-email' })
}
