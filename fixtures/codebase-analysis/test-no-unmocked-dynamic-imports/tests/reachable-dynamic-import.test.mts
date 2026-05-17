import { test } from 'vitest'
import { renderComponent } from '../src/next-dynamic-component.mts'

test('component schedules a dynamic import after render', () => {
  renderComponent()
})
