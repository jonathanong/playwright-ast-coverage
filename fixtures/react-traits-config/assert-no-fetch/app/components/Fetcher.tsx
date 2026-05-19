export default async function Fetcher() {
  let res;
  try {
    res = await fetch('/api/users');
  } catch (e) {
    return <div>Error</div>;
  }
  return <div>Users</div>;
}
