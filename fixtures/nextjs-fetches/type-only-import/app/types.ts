export type User = {
  id: string;
};
export const getUser = () => fetch('/api/type-only');
