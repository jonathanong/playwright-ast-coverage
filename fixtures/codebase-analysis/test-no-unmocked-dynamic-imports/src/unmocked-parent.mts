import { unmockedChild } from './unmocked-child.mts'

export function parent() {
  return unmockedChild()
}
