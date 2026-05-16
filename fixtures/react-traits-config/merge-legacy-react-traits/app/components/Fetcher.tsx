export default async function Fetcher() {
  const data = await fetch('/api/data');
  return <div>{data}</div>;
}
