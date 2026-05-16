export async function retryInForLoop(values: number[]) {
  for (const value of values) {
    await new Promise(resolve => setTimeout(resolve, 10))
    console.log(value)
  }
}

export async function retryInWhileLoop(count: number) {
  let i = 0
  while (i < count) {
    setTimeout(() => console.log(i), 100)
    i++
  }
}
