import openaiClient from '@services/openai-utils/client.mts'

describe('openai test', () => {
  it('has correct naming', async () => {
    const result = await openaiClient.responses.create({ model: 'gpt-4o', input: 'test' })
    expect(result).toBeDefined()
  })
})
