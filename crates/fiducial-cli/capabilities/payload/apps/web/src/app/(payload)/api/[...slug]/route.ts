import config from "@payload-config";
import {
  REST_DELETE,
  REST_GET,
  REST_OPTIONS,
  REST_PATCH,
  REST_POST,
  REST_PUT,
} from "@payloadcms/next/routes";
import type { NextRequest } from "next/server";

type RouteContext = { params: Promise<{ slug: string[] }> };

export function DELETE(req: NextRequest, ctx: RouteContext) {
  return REST_DELETE(config)(req, ctx);
}
export function GET(req: NextRequest, ctx: RouteContext) {
  return REST_GET(config)(req, ctx);
}
export function OPTIONS(req: NextRequest, ctx: RouteContext) {
  return REST_OPTIONS(config)(req, ctx);
}
export function PATCH(req: NextRequest, ctx: RouteContext) {
  return REST_PATCH(config)(req, ctx);
}
export function POST(req: NextRequest, ctx: RouteContext) {
  return REST_POST(config)(req, ctx);
}
export function PUT(req: NextRequest, ctx: RouteContext) {
  return REST_PUT(config)(req, ctx);
}
