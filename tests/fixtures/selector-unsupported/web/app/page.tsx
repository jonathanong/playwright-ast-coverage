export default function Page({ id }: { id: string }) {
  return <main><button data-testid={id}>Save</button><span data-pw={`${id}`} /></main>;
}
