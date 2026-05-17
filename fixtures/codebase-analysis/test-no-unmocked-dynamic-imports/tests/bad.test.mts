import { expect, test, vi } from 'vitest'

vi.mock('@lib/unmocked-parent.mts', () => ({
  parent: () => 'mocked',
}))

test('missing transitive mock is reported', async () => {
  const mod = await import('@lib/unmocked-parent.mts')
  expect(mod.parent()).toBe('mocked')
})

test('non literal dynamic import is reported', async () => {
  const name = '@lib/lazy.mts'
  await import(name)
})
