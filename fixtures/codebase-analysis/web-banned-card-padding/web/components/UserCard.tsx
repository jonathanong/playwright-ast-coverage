export function UserCard({ name, email }: Props) {
  return (
    <Card className="p-4 rounded shadow">
      <h2>{name}</h2>
      <p>{email}</p>
    </Card>
  );
}
