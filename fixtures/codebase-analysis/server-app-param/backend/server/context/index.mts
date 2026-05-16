import { Application } from '@modules/api-server';

// context/ directory is excluded from this rule
export function createContext(app: Application) {
  return { app };
}
