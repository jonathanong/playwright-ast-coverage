export const EMAIL_DEFAULTS = {
  attempts: 3,
  backoff: { type: 'exponential', delay: 1000, jitter: 0.5 },
}
