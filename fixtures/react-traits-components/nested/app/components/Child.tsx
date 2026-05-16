export default async function Child() {
  const data = await fetch('/api/data');
  return <div>Child</div>;
}
