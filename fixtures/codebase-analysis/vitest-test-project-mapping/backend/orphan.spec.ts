import { expect, test } from 'vitest'

test('unmapped spec file fails ownership guardrail', () => {
  expect(1 + 1).toBe(2)
})
