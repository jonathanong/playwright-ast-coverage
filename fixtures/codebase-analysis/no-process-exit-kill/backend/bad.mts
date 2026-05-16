const pid = process.pid

export function bareExit() {
  process.exit(1)
}

export function bareKill() {
  process.kill(pid, 'SIGTERM')
}

export function emitSignal() {
  process.emit('SIGTERM')
}

export function missingUnref() {
  setTimeout(() => {
    process.exit(1)
  }, 5000)
}
