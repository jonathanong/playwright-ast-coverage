export async function withinLimit() {
  await new Promise(resolve => setTimeout(resolve, 1000))
}

export async function atBoundary() {
  await new Promise(resolve => setTimeout(resolve, 5000))
}

export function shortTimeout() {
  setTimeout(() => console.log('done'), 100)
}

export async function dynamicDelay(ms: number) {
  await new Promise(resolve => setTimeout(resolve, ms))
}
