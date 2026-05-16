export function validAlternatives() {
  const obj: Record<string, string | undefined> = { a: 'hello', b: 'world' }
  const map = new Map<string, string>([['a', 'hello']])
  const set = new Set<string>(['a', 'b'])

  obj.b = undefined

  const { b: _b, ...rest } = obj
  const trimmed = rest

  map.delete('a')
  set.delete('a')

  return { trimmed, map, set }
}
