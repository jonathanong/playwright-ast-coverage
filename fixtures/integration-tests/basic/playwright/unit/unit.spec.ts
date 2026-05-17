import { test } from '@playwright/test'
import { callOpenAI } from '../../helpers/openai.mts'

test('unit playwright', async () => {})

test('playwright helper integration in unit suite', async () => {
  await callOpenAI()
})
