interface Props {
  children: React.ReactNode;
}

export function AsideColumn({ children }: Props) {
  return (
    <aside className="w-64 shrink-0 hidden lg:block">
      {children}
    </aside>
  );
}
