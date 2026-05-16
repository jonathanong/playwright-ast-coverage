export async function tooLongCallbackTimeout() {
  await new Promise(resolve => setTimeout(resolve, 6000))
}

export async function tooLongTimersPromises() {
  await setTimeout(7500)
}

export function tooLongFallback() {
  setTimeout(() => console.log('done'), 10_000)
}
