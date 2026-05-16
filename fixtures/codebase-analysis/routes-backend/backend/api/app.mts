import { createApp } from '@modules/api-server'

const cfWorkerSecret = process.env.CF_WORKER_SECRET
const isDev = process.env.NODE_ENV === 'development' || process.env.NODE_ENV === 'test'
if (!cfWorkerSecret) {
  if (!isDev) {
    throw new Error(
      'CF_WORKER_SECRET is not set — refusing to start with origin validation disabled',
    )
  }
} else if (cfWorkerSecret.length < 32) {
  if (!isDev) {
    throw new Error(
      'CF_WORKER_SECRET is too short (minimum 32 characters)',
    )
  }
}

export default createApp({
  cfWorkerSecret,
  workerSecretExemptPaths: ['/infra/ses-bounce', '/api/v1/webhooks/stripe'],
})
