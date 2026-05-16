export async function runTask() {
  await new Promise(resolve => setTimeout(resolve, 10))

  const values = [1, 2, 3]
  for (const value of values) {
    console.log(value)
  }
}

export async function singleTimeout() {
  setTimeout(() => console.log('done'), 100)
}
