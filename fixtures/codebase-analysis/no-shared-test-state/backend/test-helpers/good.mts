// No module-level mutable state — all clean

export function createUser(overrides: Record<string, unknown> = {}) {
  return { id: 1, name: 'Alice', ...overrides }
}

export function createOrder() {
  return { id: 1, userId: 1 }
}

const DB_URL = 'postgres://localhost/test'

export { DB_URL }
