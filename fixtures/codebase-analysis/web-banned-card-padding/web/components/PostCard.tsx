export function PostCard({ title, body }: Props) {
  return (
    <Card className="p-3 rounded shadow">
      <h2>{title}</h2>
      <p>{body}</p>
    </Card>
  );
}
