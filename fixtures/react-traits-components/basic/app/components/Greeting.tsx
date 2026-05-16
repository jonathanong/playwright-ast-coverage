interface Props {
  name: string;
}

export default function Greeting({ name }: Props) {
  return <div>Hello, {name}!</div>;
}
