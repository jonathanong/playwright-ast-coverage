import { psql } from '@data-stores/psql'
import { describe, it } from 'vitest'

describe('bad backend test with direct SQL', () => {
  it('calls sql directly', async () => {
    // Violation: direct SQL call in backend test
    const result = await psql.read('SELECT * FROM users')
    const written = await psql.write('INSERT INTO users VALUES ($1)', ['alice'])
    const queried = await psql.query('UPDATE users SET name = $1 WHERE id = $2', ['bob', 1])
  })
})
