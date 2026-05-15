import { getUsers } from '../../lib/api';

export default function Page() {
  getUsers();
  return <div>Users</div>;
}
