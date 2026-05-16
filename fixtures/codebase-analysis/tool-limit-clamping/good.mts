// Not in backend/tools/ — should not be flagged
export function clamp(limit: number) {
  return Math.min(limit, 100)
}
