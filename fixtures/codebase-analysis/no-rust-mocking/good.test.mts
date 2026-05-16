import { vi, describe, it, expect } from 'vitest'

describe('good test without rust mocking', () => {
  it('uses vi.fn on non-rust code', () => {
    const mockFn = vi.fn()
    mockFn.mockReturnValue(42)
    expect(mockFn()).toBe(42)
  })
})
