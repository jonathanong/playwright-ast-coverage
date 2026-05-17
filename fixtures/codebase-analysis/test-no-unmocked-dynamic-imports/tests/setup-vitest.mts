import { vi } from 'vitest'

vi.mock('@lib/setup-target.mts', () => ({
  setupValue: 'mocked',
}))
