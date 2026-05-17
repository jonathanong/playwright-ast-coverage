import { expect, test } from 'vitest'

test('setup file mock counts', async () => {
  const mod = await import('@lib/setup-target.mts')
  expect(mod.setupValue).toBe('mocked')
})
