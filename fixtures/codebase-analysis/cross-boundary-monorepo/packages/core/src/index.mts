// Scenario 1: entry point resolved via package.json exports["."]
// Scenario 6: barrel re-export — consumers of internalHelper flow through here

export { internalHelper, INTERNAL_CONST } from './internal.mts';
