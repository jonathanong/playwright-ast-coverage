// Scenarios 6: source of barrel re-export via packages/core/src/index.mts
// dependents#internalHelper should reach apps/backend/api/handler.mts via the barrel

export function internalHelper(): string {
  return 'internal';
}

export const INTERNAL_CONST = 42;
