export default /* no-mistakes: integration=openai */ function defaultCall() {
  return 'openai'
}

export const namedCall = /* no-mistakes: integration=openai */ () => 'openai'

export function namespaceCall() {
  return namedCall()
}

const aliasedCall = /* no-mistakes: integration=openai */ () => 'openai'
export { aliasedCall as reexportedCall }
export { aliasedCall as "quoted-call" }
