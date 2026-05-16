// Scenario 3: resolved via apps/backend/tsconfig.json paths: @services/topics/get → ./services/topics/get
// Scenario 6: getTopicByAny is also imported in handler.mts for symbol-level dependents

export function getTopicByAny(id: string): string {
  return id;
}

export function getTopicById(id: string): string {
  return id;
}
