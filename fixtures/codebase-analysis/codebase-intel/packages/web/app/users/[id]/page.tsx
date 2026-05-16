import Link from 'next/link';
export default function Page({ params }: { params: { id: string } }) {
  return <Link href={`/api/v1/users/${params.id}`}>User</Link>;
}
