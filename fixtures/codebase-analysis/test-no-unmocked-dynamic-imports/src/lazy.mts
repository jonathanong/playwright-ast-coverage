import { child } from './child.mts'
import type { LazyType } from './types.mts'

export function run(): LazyType {
  return child()
}
