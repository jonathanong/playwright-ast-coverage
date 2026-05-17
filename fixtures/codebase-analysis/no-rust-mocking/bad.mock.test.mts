import { vi, describe, it } from 'vitest'
import * as rustNapi from '@example/rust-napi'

// Violation: vi.mock on a Rust module
vi.mock('@example/rust-napi', () => ({
  compute: vi.fn(),
}))

describe('bad rust mocking', () => {
  it('should not spy on rust', () => {
    // Violation: vi.spyOn on a Rust import
    vi.spyOn(rustNapi, 'compute')
  })
})
