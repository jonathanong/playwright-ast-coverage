import { vi, describe, it } from 'vitest'
import * as rustNapi from '@voucha/rust-napi'

// Violation: vi.mock on a Rust module
vi.mock('@voucha/rust-napi', () => ({
  compute: vi.fn(),
}))

describe('bad rust mocking', () => {
  it('should not spy on rust', () => {
    // Violation: vi.spyOn on a Rust import
    vi.spyOn(rustNapi, 'compute')
  })
})
