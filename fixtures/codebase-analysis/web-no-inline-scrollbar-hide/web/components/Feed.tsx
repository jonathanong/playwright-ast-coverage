export default function Feed({ items }: Props) {
  return (
    <div
      className="overflow-auto"
      style={{ scrollbarWidth: 'none' }}
    >
      {items.map((item) => (
        <div key={item.id}>{item.title}</div>
      ))}
    </div>
  );
}
