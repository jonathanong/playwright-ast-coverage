export default async function DynamicPage({ params }: { params: { path: string } }) {
  return <div>{params.path}</div>;
}
