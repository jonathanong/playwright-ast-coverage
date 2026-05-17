export async function GET() {
  return Response.json({ ok: true });
}

export async function OPTIONS() {
  return new Response(null, { status: 204 });
}

export const runtime = 'edge';
