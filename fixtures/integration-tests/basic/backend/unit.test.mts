import { expect, test } from 'vitest'
import { callOpenAI, callOpenAIExpression } from '../helpers/openai.mts'

test('mocked unit test', () => {
  expect(1).toBe(1)
})

test('direct integration in unit suite', /* no-mistakes: integration=openai */ async () => {
  expect(await Promise.resolve('ok')).toBe('ok')
})

test('helper integration in unit suite', async () => {
  await callOpenAI()
})

test('expression helper integration in unit suite', async () => {
  await callOpenAIExpression()
})

describe.skip('skipped integration group', () => {
  test('skipped helper integration in unit suite', async () => {
    await callOpenAI()
  })
})
