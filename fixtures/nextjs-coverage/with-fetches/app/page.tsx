export default async function Home() {
  await fetch('/api/health');
  return null;
}
