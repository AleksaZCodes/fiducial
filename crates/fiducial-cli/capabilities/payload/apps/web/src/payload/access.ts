import { type Grant, type Principal, type Resource, can } from "@fiducial/identity";
import type { Access, AccessArgs } from "payload";
import type { PayloadRole } from "../generated/payload";

type PayloadUser = {
  id: string;
  role: PayloadRole;
  collection: "users";
};

function grantsFor(user: PayloadUser): Grant[] {
  const principal: Principal = { kind: "user", id: user.id };
  const resource: Resource = { kind: "platform" };
  const roleMap: Record<PayloadRole, Grant["role"]> = {
    owner: "owner",
    staff: "member",
    "agency-admin": "viewer",
  };
  return [{ principal, resource, role: roleMap[user.role] }];
}

function principalFrom(user: PayloadUser | null | undefined): Principal {
  if (!user) return { kind: "anonymous" };
  if (user.role === "agency-admin") {
    return { kind: "service", id: user.id };
  }
  return { kind: "user", id: user.id };
}

export const isOwnerOrStaff: Access = ({ req }: AccessArgs) => {
  const user = req.user as PayloadUser | null;
  if (!user) return false;
  const principal = principalFrom(user);
  const grants = grantsFor(user);
  const resource: Resource = { kind: "platform" };
  return can(principal, "write", resource, grants);
};

export const isOwner: Access = ({ req }: AccessArgs) => {
  const user = req.user as PayloadUser | null;
  if (!user) return false;
  const principal = principalFrom(user);
  const grants = grantsFor(user);
  const resource: Resource = { kind: "platform" };
  return can(principal, "admin", resource, grants);
};

export const isAgencyAdmin: Access = ({ req }: AccessArgs) => {
  const user = req.user as PayloadUser | null;
  if (!user) return false;
  return user.role === "agency-admin";
};

export const isAuthenticated: Access = ({ req }: AccessArgs) => {
  return Boolean(req.user);
};
