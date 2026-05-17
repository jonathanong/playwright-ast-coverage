import defaultCall, { namedCall as renamedCall, reexportedCall } from './helpers'
import * as helperNamespace from './helpers'
import defaultArrow from './default-arrow'
import './side-effect'

/* no-mistakes: integration=openai */
function declaredIntegration() {
  return 'openai'
}

const arrowIntegration = /* no-mistakes: integration=anthropic */ () => 'anthropic'
const functionIntegration = /* no-mistakes: integration=gemini */ function () {
  return 'gemini'
}
const { ignored } = { ignored: true }

export function exportedDeclared() {
  return declaredIntegration()
}

export const exportedArrow = () => arrowIntegration()
export const exportedFunction = function () {
  return functionIntegration()
}
export const { ignoredExport } = { ignoredExport: true }
export class ExportedClass {}

describe(`outer`, () => {
  it(`uses declared function`, function () {
    exportedDeclared()
  })

  test('uses namespace function', () => {
    helperNamespace.namespaceCall()
  })

test('uses default import', () => {
    defaultCall()
  })

  test('uses named import', () => {
    renamedCall()
  })

  test('uses re-exported function', () => {
    reexportedCall()
  })

  test('uses default arrow function', () => {
    defaultArrow()
  })

  test('uses unresolved local', () => {
    unresolvedLocal()
  })

  test.skip('skipped integration', () => {
    declaredIntegration()
  })

  test.fixme('fixme integration', () => {
    declaredIntegration()
  })

  test.describe('nested', () => {
    test('nested no callback')
  })
})
