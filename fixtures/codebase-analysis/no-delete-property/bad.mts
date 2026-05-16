export function deleteProperties() {
  const obj: Record<string, string> = { a: 'hello', b: 'world' }
  const key = 'b'

  delete obj.a
  delete obj.b
  delete obj[key]
  delete obj['b']
}
