import { a } from './source.mts'

// Valid: function call
const getUserEntityCacheKeys = () => ['user:1', 'user:2']
export const getUserCacheKeys = getUserEntityCacheKeys()

// Valid: property access
const obj = { method: () => 'value' }
export const getMethod = obj.method

// Valid: arrow function
export const compute = () => 42

// Valid: string literal
export const API_KEY = 'secret-key-123'

// Valid: number literal
export const MAX_RETRIES = 3

// Valid: direct function export
export function fetchData() {
  return []
}

// Valid: non-renamed specifier export
const foo = 1
const bar = 2
export { foo, bar }

// Valid: type-only alias — not a value export
type Alias = string
export type { Alias as StringAlias }

// Valid: individual type specifier
type MyType = number
export { type MyType as NumType }

// Valid: namespace re-export (not a symbol alias)
export * as ns from './source.mts'

// Valid: re-export without renaming
export { a } from './source.mts'
