'use client';

export default function Home() {
  const handleClick = async () => {
    await fetch('/api/health');
  };
  return <button onClick={handleClick}>Click</button>;
}
