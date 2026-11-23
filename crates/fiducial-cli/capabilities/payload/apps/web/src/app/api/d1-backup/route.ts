import { NextRequest, NextResponse } from "next/server";

export const dynamic = "force-dynamic";

interface WorkerEnv {
  BACKUP_TOKEN?: string;
  DB?: { dump(): Promise<ArrayBuffer> };
  BUCKET?: { put(key: string, value: ArrayBuffer | ReadableStream): Promise<void> };
}

export async function POST(req: NextRequest): Promise<NextResponse> {
  const env = process.env as unknown as WorkerEnv;

  const token = env.BACKUP_TOKEN;
  if (token) {
    const auth = req.headers.get("authorization") ?? "";
    if (auth !== `Bearer ${token}`) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }
  }

  if (!env.DB) {
    return NextResponse.json({ error: "DB binding not available" }, { status: 500 });
  }
  if (!env.BUCKET) {
    return NextResponse.json({ error: "BUCKET binding not available" }, { status: 500 });
  }

  const dump = await env.DB.dump();
  const date = new Date().toISOString().slice(0, 10);
  const key = `backup/${date}.sql`;
  await env.BUCKET.put(key, dump);

  return NextResponse.json({ ok: true, key, bytes: dump.byteLength });
}
