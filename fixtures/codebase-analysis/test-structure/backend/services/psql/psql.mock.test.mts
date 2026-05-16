import { vi, describe, it } from 'vitest'

vi.mock('./psql-module.mts', () => ({
  read: vi.fn(),
}))

describe('valid mock test', () => {
  it('mocks properly', () => {
    vi.mock('./psql-module.mts', () => ({
      read: vi.fn(),
    }))
  })
})
