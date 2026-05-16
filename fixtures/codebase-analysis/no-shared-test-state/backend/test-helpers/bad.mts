import { createClient } from '@data-stores/psql'

// module-level let — violation
let counter = 0

// module-level const [] — violation
const collected: string[] = []

// exported function with "shared" in name — violation
export function createSharedUser() {
  return { id: 1 }
}

// module-level const new Array() — violation
const items = new Array<string>()

export function doSomething() {
  counter++
  collected.push('x')
}
