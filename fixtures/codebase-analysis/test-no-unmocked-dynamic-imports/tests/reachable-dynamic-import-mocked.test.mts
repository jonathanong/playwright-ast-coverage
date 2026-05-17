import { test, vi } from 'vitest'
import { renderComponent } from '../src/next-dynamic-component.mts'

vi.mock('../src/dynamic-leaf.mts')

test('component dynamic import is mocked', () => {
  renderComponent()
})
