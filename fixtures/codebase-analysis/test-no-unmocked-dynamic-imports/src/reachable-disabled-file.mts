// guardrails-disable-file test-no-unmocked-dynamic-imports
export function disabledFileImport() {
  return import('./dynamic-leaf.mts')
}
