import { expect, test } from 'vitest'
import { callAnthropic, callOpenAI } from '../helpers/openai.mts'

test('plain test allowed in non-strict suite', () => {
  expect(1).toBe(1)
})

test('openai allowed in non-strict suite', async () => {
  await callOpenAI()
})

test('wrong integration still fails in non-strict suite', async () => {
  await callAnthropic()
})

test('wrong integration fails even when allowed integration is also called', async () => {
  await callOpenAI()
  await callAnthropic()
})
