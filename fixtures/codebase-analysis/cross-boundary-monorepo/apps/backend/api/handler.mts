// Scenario 1: @scope/core resolves to packages/core/src/index.mts via workspace exports["."]
// Scenario 3: @services/topics/get resolves via apps/backend/tsconfig.json paths alias
// Scenario 5: import type (relative path) produces TypeImport edge, not Import
// Scenario 6: internalHelper is imported via @scope/core barrel — enables symbol-level dependents

import { internalHelper } from '@scope/core';
import { getTopicByAny } from '@services/topics/get';
import type { TopicQuery } from '../services/topics/types.mts';

export function handleRequest(query: TopicQuery): string {
  return internalHelper() + getTopicByAny(query.id);
}
