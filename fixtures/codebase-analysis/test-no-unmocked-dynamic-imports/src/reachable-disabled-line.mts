export function disabledLineImport() {
  // guardrails-disable-next-line test-no-unmocked-dynamic-imports
  return import('./dynamic-leaf.mts')
}
