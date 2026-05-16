import { vi, describe, it } from 'vitest'

// Violation: mocking in a regular .test.mts file
vi.mock('./some-module.mts', () => ({
  doThing: vi.fn(),
}))

describe('bad test using mocks', () => {
  it('uses vi.spyOn', () => {
    const obj = { method: () => 42 }
    vi.spyOn(obj, 'method')
  })
})
