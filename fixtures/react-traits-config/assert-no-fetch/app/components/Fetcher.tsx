export default async function Fetcher() {
  const res = await fetch('/api/users');
  return <div>Users</div>;
}
