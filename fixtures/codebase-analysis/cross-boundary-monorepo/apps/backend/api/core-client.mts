// Scenario 7: @core/* resolves via @core/* paths inherited from tsconfig.base.json through extends.
// Used by scenario7_tsconfig_extends_paths_inherited to test that tsconfig.extends is followed.

import { internalHelper } from '@core/internal';

export function useCore(): string {
  return internalHelper();
}
