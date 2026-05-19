'use client';

export default function Home() {
  const handleClick = async () => {
    await fetch('/api/health');
  };
  return <button type="button" onClick={() => { void handleClick(); }}>Click</button>;
}
