export default function Page({ params }: { params: { id: string } }) {
  return <main><article data-testid={`user-${params.id}`} /><button data-pw={`user-${params.id}-button`}>Open</button></main>;
}
