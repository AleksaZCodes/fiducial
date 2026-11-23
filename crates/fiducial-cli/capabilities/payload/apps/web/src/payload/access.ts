import { type Grant, type Principal, type Resource, can } from "@fiducial/identity";
import type { Access, AccessArgs } from "payload";
import type { PayloadRole } from "../generated/payload";

export type RequestUser = {
  id: string;
  role: PayloadRole;
};

export function principalFrom(user: RequestUser): Principal {
  if (user.role === "agency-admin") return { kind: "service", id: user.id };
  return { kind: "user", id: user.id };
}

export function grantsFor(user: RequestUser): Grant[] {
  const principal = principalFrom(user);
  const resource: Resource = { kind: "platform" };
  const roleMap: Record<PayloadRole, Grant["role"]> = {
    owner: "owner",
    staff: "member",
    "agency-admin": "viewer",
  };
  return [{ principal, resource, role: roleMap[user.role] }];
}

export const isOwnerOrStaff: Access = ({ req }: AccessArgs) => {
  const user = req.user as RequestUser | null;
  if (!user) return false;
  const resource: Resource = { kind: "platform" };
  return can(principalFrom(user), "write", resource, grantsFor(user));
};

export const isOwner: Access = ({ req }: AccessArgs) => {
  const user = req.user as RequestUser | null;
  if (!user) return false;
  const resource: Resource = { kind: "platform" };
  return can(principalFrom(user), "admin", resource, grantsFor(user));
};

export const isAgencyAdmin: Access = ({ req }: AccessArgs) => {
  const user = req.user as RequestUser | null;
  if (!user) return false;
  return user.role === "agency-admin";
};

export const isAuthenticated: Access = ({ req }: AccessArgs) => {
  return Boolean(req.user);
};
