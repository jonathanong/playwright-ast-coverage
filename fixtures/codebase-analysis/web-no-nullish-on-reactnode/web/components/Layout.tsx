interface Props {
  footer?: React.ReactNode;
}

export default function Layout({ footer }: Props) {
  return (
    <div className="layout">
      <main className="content">
        <slot />
      </main>
      <Panel footer={footer ?? <DefaultFooter />} />
    </div>
  );
}
