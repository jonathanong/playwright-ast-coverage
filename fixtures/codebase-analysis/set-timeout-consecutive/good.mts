export async function singleTimeout() {
  await new Promise(resolve => setTimeout(resolve, 1000))
}

export async function timeoutThenOtherWork() {
  await new Promise(resolve => setTimeout(resolve, 1000))
  await Promise.resolve()
  await new Promise(resolve => setTimeout(resolve, 1000))
}

export async function notConsecutive() {
  await new Promise(resolve => setTimeout(resolve, 100))
  const x = await Promise.resolve(42)
  await new Promise(resolve => setTimeout(resolve, 100))
  console.log(x)
}
