export function forcedExitFallback() {
  setTimeout(() => {
    process.exit(1)
  }, 5000).unref()
}
