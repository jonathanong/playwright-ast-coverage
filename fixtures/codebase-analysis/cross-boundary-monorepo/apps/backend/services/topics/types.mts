// Scenario 5: type-only export imported via relative path in handler.mts
// import type { TopicQuery } from '../services/topics/types.mts' produces TypeImport, not Import

export interface TopicQuery {
  id: string;
  limit: number;
}

export type TopicId = string;
