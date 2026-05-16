interface Props {
  footer?: React.ReactNode;
}

export default function Sidebar({ footer }: Props) {
  return (
    <div className="sidebar">
      <Panel footer={footer !== undefined ? footer : <DefaultFooter />} />
    </div>
  );
}
