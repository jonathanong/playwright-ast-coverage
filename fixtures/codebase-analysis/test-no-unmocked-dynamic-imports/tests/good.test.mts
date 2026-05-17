import { expect, test, vi } from 'vitest'

vi.mock('@lib/lazy.mts', () => ({
  run: () => 'mocked',
}))
vi.mock('@lib/child.mts', () => ({
  child: () => 'mocked',
}))

test('dynamic import closure is mocked', async () => {
  const mod = await import('@lib/lazy.mts')
  expect(mod.run()).toBe('mocked')
})
