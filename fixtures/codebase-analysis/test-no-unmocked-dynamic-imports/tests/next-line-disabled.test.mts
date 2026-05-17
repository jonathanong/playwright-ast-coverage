import { test } from 'vitest'

test('next-line disable suppresses one import', async () => {
  // guardrails-disable-next-line test-no-unmocked-dynamic-imports
  await import('@lib/disabled.mts')
})
