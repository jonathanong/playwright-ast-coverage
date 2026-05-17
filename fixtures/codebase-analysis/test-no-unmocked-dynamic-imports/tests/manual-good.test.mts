import { expect, test } from 'vitest'

test('manual mock counts', async () => {
  const mod = await import('@lib/manual.mts')
  expect(mod.manual()).toBe('mocked')
})
