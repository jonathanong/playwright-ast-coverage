export async function consecutiveCallbackTimeouts() {
  await new Promise(resolve => setTimeout(resolve, 1000))
  await new Promise(resolve => setTimeout(resolve, 1000))
}

export async function consecutiveDirectTimeouts() {
  await setTimeout(500)
  await setTimeout(500)
}

export async function tripleConsecutive() {
  await new Promise(resolve => setTimeout(resolve, 100))
  await new Promise(resolve => setTimeout(resolve, 200))
  await new Promise(resolve => setTimeout(resolve, 300))
}
