test('jest setup file mock counts', async () => {
  const mod = await import('@lib/jest-setup-target.mts')
  expect(mod.jestSetupValue).toBe('mocked')
})
