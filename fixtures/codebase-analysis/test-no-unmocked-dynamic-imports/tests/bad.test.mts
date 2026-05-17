import { expect, test } from 'vitest'

test('missing transitive mock is reported', async () => {
  const mod = await import('@lib/unmocked-parent.mts')
  expect(mod.parent()).toBe('parent')
})

test('non literal dynamic import is reported', async () => {
  const name = '@lib/lazy.mts'
  await import(name)
})
