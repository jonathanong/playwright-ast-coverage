export default async function Fetcher() {
  try {
    await fetch('/api/users');
  } catch {
    return <div>Error</div>;
  }
  return <div>Users</div>;
}
