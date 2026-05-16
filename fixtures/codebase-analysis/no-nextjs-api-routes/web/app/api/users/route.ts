export async function GET(req: Request) {
  return Response.json({ users: [] })
}

export async function POST(req: Request) {
  const body = await req.json()
  return Response.json({ created: body })
}
