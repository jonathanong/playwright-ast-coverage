import { expect, test } from 'vitest'
import { callOpenAI } from '../helpers/openai.mts'

test('direct openai integration', /* no-mistakes: integration=openai */ async () => {
  expect(await Promise.resolve('ok')).toBe('ok')
})

test('helper openai integration', async () => {
  await callOpenAI()
})

test('strict suite requires annotation', async () => {
  expect(1).toBe(1)
})
