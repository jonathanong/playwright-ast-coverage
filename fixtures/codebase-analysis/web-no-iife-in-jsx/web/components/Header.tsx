export default function Header({ loading, title }: Props) {
  return (
    <header className="header">
      {loading ? <Skeleton /> : <h1>{title}</h1>}
    </header>
  );
}
