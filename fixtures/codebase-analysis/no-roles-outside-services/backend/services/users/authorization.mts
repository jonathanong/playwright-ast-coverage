import type { User } from '@app/models';

// .roles access inside services is allowed
export function isAdmin(user: User): boolean {
  return user.roles.includes('administrator');
}
