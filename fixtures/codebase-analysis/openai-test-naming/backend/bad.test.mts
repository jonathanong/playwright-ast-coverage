import openaiClient from '@services/openai-utils/client.mts'

describe('email sender', () => {
  it('sends an email', async () => {
    const result = await openaiClient.responses.create({ model: 'gpt-4o', input: 'test' })
    expect(result).toBeDefined()
  })
})
