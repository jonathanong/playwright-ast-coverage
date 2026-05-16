// Scenario 4: @scope/types resolves via apps/web/tsconfig.json paths to packages/core/src/types.mts
// cross-package alias: web package imports a type from the core package via tsconfig paths

import type { CoreConfig } from '@scope/types';

export function Page(props: { config: CoreConfig }): null {
  return null;
}
