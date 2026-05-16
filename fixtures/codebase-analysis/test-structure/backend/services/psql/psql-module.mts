/** guardrails: allow-mocking */
export function read(query: string): Promise<unknown[]> {
  return Promise.resolve([])
}

export function write(query: string): Promise<void> {
  return Promise.resolve()
}
