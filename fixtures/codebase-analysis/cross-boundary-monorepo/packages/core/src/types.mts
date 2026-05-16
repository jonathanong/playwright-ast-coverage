// Scenarios 4: exported as a type-only import in web/pages/index.tsx
// Scenario 4: web/tsconfig.json maps @scope/types → ../../packages/core/src/types.mts

export interface CoreConfig {
  debug: boolean;
  timeout: number;
}

export type CoreId = string;
