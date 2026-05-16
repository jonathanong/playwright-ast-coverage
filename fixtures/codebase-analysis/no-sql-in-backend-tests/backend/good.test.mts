import { createTestUser } from '../test-helpers/users.mts'
import { describe, it, expect } from 'vitest'

describe('good backend test using helper', () => {
  it('uses test helpers instead of direct SQL', async () => {
    const user = await createTestUser({ name: 'Alice' })
    expect(user.id).toBeDefined()
  })
})
