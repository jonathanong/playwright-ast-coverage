export default function Feed({ loading, posts }: Props) {
  return (
    <div className="feed">
      {(() => {
        if (loading) return <Spinner />;
        return <Posts items={posts} />;
      })()}
    </div>
  );
}
