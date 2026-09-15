/**
 * @fiducial/identity — one identity model, at every level.
 *
 * Mirrors the Rust `fiducial-identity` crate. The Rust side runs in firmware,
 * the desktop backend and the CLI; this side runs in the Worker and the app.
 * Both answer the same question — *may this actor do this to this thing?* —
 * and they are held to the same answer by `docs/identity/vectors.json`, a
 * decision table generated from the Rust crate and asserted by both suites.
 *
 * Ids are lowercase hex strings here rather than byte arrays: that is what
 * crosses a wire, lands in a database column and appears in the vectors, and
 * converting at each boundary instead would be three places to get it wrong.
 * The Rust side holds the same ids as fixed-width bytes because it must run
 * without an allocator.
 */

/** A 16-byte user id, as 32 lowercase hex characters. */
export type UserId = string;
/** An 8-byte device id, as 16 lowercase hex characters. */
export type DeviceId = string;
/** An 8-byte service id, as 16 lowercase hex characters. */
export type ServiceId = string;

/** Who is acting — at any level of the platform. */
export type Principal =
  | { kind: "anonymous" }
  | { kind: "user"; id: UserId }
  | { kind: "device"; id: DeviceId }
  | { kind: "service"; id: ServiceId };

/** What is being acted upon. */
export type Resource =
  | { kind: "device"; id: DeviceId }
  | { kind: "user"; id: UserId }
  | { kind: "platform" };

/** What a principal is trying to do, ordered by increasing power. */
export type Action = "read" | "write" | "admin";

/** How much a principal may do with a resource. */
export type Role = "viewer" | "member" | "admin" | "owner";

/** One principal's role over one resource. */
export interface Grant {
  principal: Principal;
  resource: Resource;
  role: Role;
}

const ACTION_RANK: Record<Action, number> = { read: 0, write: 1, admin: 2 };
const ROLE_RANK: Record<Role, number> = {
  viewer: 0,
  member: 1,
  admin: 2,
  owner: 3,
};
/** The most powerful action each role permits. */
const ROLE_PERMITS: Record<Role, Action> = {
  viewer: "read",
  member: "write",
  admin: "admin",
  owner: "admin",
};

/** `true` when `role` permits `action`. */
export function roleAllows(role: Role, action: Action): boolean {
  return ACTION_RANK[action] <= ACTION_RANK[ROLE_PERMITS[role]];
}

/** An all-zero id is the uninitialized sentinel at every level. */
function isZeroId(id: string): boolean {
  return id.length === 0 || /^0+$/.test(id);
}

/**
 * `true` when this principal is a real, initialized identity.
 *
 * An unprovisioned device that has not read its UID out of OTP would
 * otherwise authenticate as "device zero" — an identity every such device
 * shares.
 */
export function isIdentified(principal: Principal): boolean {
  return principal.kind !== "anonymous" && !isZeroId(principal.id);
}

function sameResource(a: Resource, b: Resource): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "platform" || b.kind === "platform") return true;
  return (a as { id: string }).id === (b as { id: string }).id;
}

function samePrincipal(a: Principal, b: Principal): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "anonymous" || b.kind === "anonymous") return true;
  return (a as { id: string }).id === (b as { id: string }).id;
}

/** `true` when this grant speaks to `principal` acting on `resource`. */
function covers(grant: Grant, principal: Principal, resource: Resource): boolean {
  return (
    samePrincipal(grant.principal, principal) &&
    (sameResource(grant.resource, resource) || grant.resource.kind === "platform")
  );
}

/**
 * May `principal` perform `action` on `resource`, given `grants`?
 *
 * Deny is the default: an empty grant table permits nothing. Two rules hold
 * before any grant is consulted:
 *
 * 1. An unidentified principal is refused — anonymous, or an all-zero id.
 * 2. A device may always read itself, with no grant. Requiring one would mean
 *    every device needs a provisioning round trip before it can say anything
 *    at all, including the "I am here, I am unclaimed" message provisioning
 *    itself depends on. It is a *read*: a device still may not write itself
 *    unguarded, or a compromised one could rewrite its own configuration and
 *    call it self-service.
 */
export function can(
  principal: Principal,
  action: Action,
  resource: Resource,
  grants: readonly Grant[],
): boolean {
  if (!isIdentified(principal)) return false;

  if (
    principal.kind === "device" &&
    resource.kind === "device" &&
    principal.id === resource.id &&
    action === "read"
  ) {
    return true;
  }

  return grants.some(
    (g) => covers(g, principal, resource) && roleAllows(g.role, action),
  );
}

/**
 * The strongest role `principal` holds over `resource`, if any.
 *
 * A UI asks a different question than a gate does: "what may I show them?"
 * needs the role, not one verdict.
 */
export function effectiveRole(
  principal: Principal,
  resource: Resource,
  grants: readonly Grant[],
): Role | null {
  if (!isIdentified(principal)) return null;
  let best: Role | null = null;
  for (const g of grants) {
    if (!covers(g, principal, resource)) continue;
    if (best === null || ROLE_RANK[g.role] > ROLE_RANK[best]) best = g.role;
  }
  return best;
}

// ── Boundary conversions ─────────────────────────────────────────────────────

/**
 * Turn an auth vendor's UUID subject into a `UserId`.
 *
 * The one place a UUID string becomes an id: hyphens dropped, lowercased, so
 * `"11111111-2222-…"` and `"11111111-2222-…"` in different cases are one
 * identity rather than two. Mirrors `UserId::parse_uuid` in Rust, which
 * produces the same 16 bytes this hex spells.
 */
export function userIdFromUuid(uuid: string): UserId {
  const hex = uuid.replace(/-/g, "").toLowerCase();
  if (!/^[0-9a-f]{32}$/.test(hex)) {
    throw new Error(`userIdFromUuid: not a UUID: ${uuid}`);
  }
  return hex;
}

/** The principal for a signed-in user, from their vendor UUID. */
export function userPrincipal(uuid: string): Principal {
  return { kind: "user", id: userIdFromUuid(uuid) };
}

/** The principal for a device, from its 8-byte id in hex. */
export function devicePrincipal(id: DeviceId): Principal {
  return { kind: "device", id: id.toLowerCase() };
}

/** The principal for a service. */
export function servicePrincipal(id: ServiceId): Principal {
  return { kind: "service", id: id.toLowerCase() };
}

/** Nobody has authenticated. */
export const ANONYMOUS: Principal = { kind: "anonymous" };

/**
 * The principal for an auth session — the whole bridge from the `auth`
 * adapter contract to identity.
 *
 * ```ts
 * const session = await auth.getSession();          // @fiducial/adapters
 * const who = principalFromSession(session);        // @fiducial/identity
 * if (!can(who, "write", thermostat, grants)) return forbidden();
 * ```
 *
 * Structurally typed on purpose: it takes `{ user: { id } }`, not an
 * `AuthSession`, so this package depends on nothing and works with any
 * vendor's session shape. A `null` session — nobody signed in, or a token
 * that failed verification — is `ANONYMOUS`, which `can()` refuses outright.
 */
export function principalFromSession(
  session: { user: { id: string } } | null | undefined,
): Principal {
  if (!session?.user?.id) return ANONYMOUS;
  return userPrincipal(session.user.id);
}

// ── Grant storage ────────────────────────────────────────────────────────────

export type { GrantDatabase, GrantStore } from "./grants.js";
export { MemoryGrantStore, SqlGrantStore } from "./grants.js";
