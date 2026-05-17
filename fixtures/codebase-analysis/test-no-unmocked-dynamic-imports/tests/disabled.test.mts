// guardrails-disable-file test-no-unmocked-dynamic-imports
import { test } from 'vitest'

test('file disable suppresses findings', async () => {
  await import('@lib/disabled.mts')
})
