export default function DashboardPage() {
  return (
    <div className="flex gap-4">
      <aside className="w-64 shrink-0">
        <nav>
          <a href="/dashboard">Overview</a>
          <a href="/dashboard/stats">Stats</a>
        </nav>
      </aside>
      <main className="flex-1">
        <h1>Dashboard</h1>
      </main>
    </div>
  );
}
