/**
 * Grant storage — where the permission rows live.
 *
 * `can()` takes a list of grants and answers yes or no. This is the half that
 * produces the list. Without it the rule was real but unusable: every caller
 * would have had to hand-assemble the rows on every request.
 *
 * A grant is one row — *who, over what, how much*:
 *
 * | principal    | resource           | role   |
 * |--------------|--------------------|--------|
 * | `user:alice` | `device:thermostat`| owner  |
 * | `user:bob`   | `device:thermostat`| viewer |
 *
 * ## Why this is not a new adapter contract
 *
 * Grants are rows in whatever database the product already selected. So this
 * is a **consumer** of the `database` contract, not a ninth contract beside
 * it: `SqlGrantStore` works on D1 today and on any future database vendor
 * for free, because it only ever calls `query`/`execute`.
 *
 * It takes that database *structurally* — a `{ query, execute }` shape rather
 * than an imported `Database` — for the same reason `principalFromSession`
 * takes an id rather than an `AuthSession`: `@fiducial/identity` depends on
 * nothing, and any implementation of the contract satisfies this by having
 * the right methods.
 *
 * ## Rust
 *
 * There is no Rust counterpart. `fiducial-identity` is `no_std` and a device
 * cannot run SQL — a device receives the grants it needs rather than querying
 * for them. `can()` stays the shared half; storage is a server concern.
 */

import type { Grant, Principal, Resource, Role } from "./index.js";

/**
 * The slice of the `database` contract this needs.
 *
 * Structurally identical to `@fiducial/adapters`' `Database` for these two
 * methods, so passing a real `Database` just works — without this package
 * importing that one.
 */
export interface GrantDatabase {
  query(
    sql: string,
    params: Array<string | number | null>,
  ): Promise<Array<{ get(column: string): unknown }>>;
  execute(
    sql: string,
    params: Array<string | number | null>,
  ): Promise<{ rowsAffected: number }>;
}

/**
 * Which SQL dialect the database speaks.
 *
 * Not a style preference — the two are different languages at the point this
 * package touches them. A bound parameter is `?` on SQLite and `$1` on
 * Postgres, and each engine rejects the other's spelling outright.
 *
 * This is **derived, not declared**: `fid derive` resolves it from `[adapters]
 * database` and writes it to `src/identity.generated.ts`, next to the table
 * name and the migration that was generated for the same dialect. Import it
 * from there rather than retyping it here.
 */
export type SqlDialect = "sqlite" | "postgres";

/** Construction facts `fid derive` writes to `src/identity.generated.ts`. */
export interface SqlGrantStoreOptions {
  /** `[identity] table`. Defaults to `grants`. */
  table?: string;
  /** Resolved from `[adapters] database`. Defaults to `sqlite`. */
  dialect?: SqlDialect;
}

/** Reading and writing the permission rows. */
export interface GrantStore {
  /**
   * Every grant held by `principal` — the list `can()` wants.
   *
   * Includes platform-scoped grants, because `can()` treats those as covering
   * every resource. Fetch this **once per request** and pass the result to as
   * many `can()` calls as you need; one query per authorization check turns a
   * cheap comparison into a round trip.
   */
  grantsFor(principal: Principal): Promise<Grant[]>;

  /**
   * Every grant over `resource` — who can see this thing, and how much.
   *
   * The question a sharing UI asks, which `grantsFor` cannot answer.
   */
  grantsOn(resource: Resource): Promise<Grant[]>;

  /** Give `principal` this `role` over `resource`, replacing any existing role. */
  grant(principal: Principal, resource: Resource, role: Role): Promise<void>;

  /** Remove `principal`'s grant over `resource`. Succeeds if there was none. */
  revoke(principal: Principal, resource: Resource): Promise<void>;
}

// ── Row ↔ value conversions ─────────────────────────────────────────────────

/** `platform` has no id; every other kind does. `''` is its stored form. */
function resourceId(resource: Resource): string {
  return resource.kind === "platform" ? "" : resource.id;
}

function toResource(kind: string, id: string): Resource {
  if (kind === "platform") return { kind: "platform" };
  return { kind: kind as "device" | "user", id };
}

function toPrincipal(kind: string, id: string): Principal {
  if (kind === "anonymous") return { kind: "anonymous" };
  return { kind: kind as "user" | "device" | "service", id };
}

function str(row: { get(column: string): unknown }, column: string): string {
  return String(row.get(column) ?? "");
}

function rowToGrant(row: { get(column: string): unknown }): Grant {
  return {
    principal: toPrincipal(str(row, "principal_kind"), str(row, "principal_id")),
    resource: toResource(str(row, "resource_kind"), str(row, "resource_id")),
    role: str(row, "role") as Role,
  };
}

// ── SQL implementation ───────────────────────────────────────────────────────

/**
 * Grant storage in the product's own relational database.
 *
 * The table is generated by `fid derive` (the `fid-identity` executor) from
 * the identity model, so its `CHECK` constraints list exactly the principal
 * kinds, resource kinds and roles this package defines. Adding a `Role` and
 * forgetting the schema is not possible: the schema is derived from the
 * model, and `fid derive --check` fails when it is stale.
 */
export class SqlGrantStore implements GrantStore {
  private readonly table: string;
  private readonly dialect: SqlDialect;

  constructor(
    private readonly db: GrantDatabase,
    options: SqlGrantStoreOptions = {},
  ) {
    const table = options.table ?? "grants";
    this.table = table;
    this.dialect = options.dialect ?? "sqlite";
    // The table name reaches SQL by interpolation — parameters cannot bind an
    // identifier — so it is validated here rather than trusted. It comes from
    // `[identity] table` in fiducial.toml, which is not user input, but "not
    // user input today" is exactly how injection sites are introduced.
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(table)) {
      throw new Error(
        `SqlGrantStore: invalid table name ${JSON.stringify(table)} — ` +
          "letters, digits and underscore only",
      );
    }
  }

  /**
   * Renumber `?` placeholders for the dialect.
   *
   * The queries below are written once, in SQLite's spelling, and rewritten
   * for Postgres — which numbers its parameters (`$1`, `$2`) and rejects `?`
   * with a syntax error. Writing each query twice would be two declarations
   * of one query, and they would drift.
   *
   * Textual rewriting is safe *here* specifically: every query in this class
   * is a literal in this file, and none contains a `?` outside a placeholder.
   * It is not a general-purpose SQL translator and must not be used as one.
   */
  private sql(query: string): string {
    if (this.dialect !== "postgres") return query;
    let n = 0;
    return query.replace(/\?/g, () => `$${++n}`);
  }

  async grantsFor(principal: Principal): Promise<Grant[]> {
    // Anonymous can hold no grant — the schema refuses to store one, and
    // `can()` refuses it anyway. Not querying says so without a round trip.
    if (principal.kind === "anonymous") return [];

    const rows = await this.db.query(
      this.sql(
        `SELECT principal_kind, principal_id, resource_kind, resource_id, role
           FROM ${this.table}
          WHERE principal_kind = ? AND principal_id = ?`,
      ),
      [principal.kind, principal.id],
    );
    return rows.map(rowToGrant);
  }

  async grantsOn(resource: Resource): Promise<Grant[]> {
    const rows = await this.db.query(
      this.sql(
        `SELECT principal_kind, principal_id, resource_kind, resource_id, role
           FROM ${this.table}
          WHERE resource_kind = ? AND resource_id = ?`,
      ),
      [resource.kind, resourceId(resource)],
    );
    return rows.map(rowToGrant);
  }

  async grant(
    principal: Principal,
    resource: Resource,
    role: Role,
  ): Promise<void> {
    if (principal.kind === "anonymous") {
      throw new Error(
        "SqlGrantStore.grant: anonymous can hold no grant — `can()` refuses " +
          "an unidentified principal, so the row could never be honoured",
      );
    }
    await this.db.execute(
      this.sql(
        `INSERT INTO ${this.table}
           (principal_kind, principal_id, resource_kind, resource_id, role, granted_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (principal_kind, principal_id, resource_kind, resource_id)
         DO UPDATE SET role = excluded.role, granted_at = excluded.granted_at`,
      ),
      [
        principal.kind,
        principal.id,
        resource.kind,
        resourceId(resource),
        role,
        new Date().toISOString(),
      ],
    );
  }

  async revoke(principal: Principal, resource: Resource): Promise<void> {
    if (principal.kind === "anonymous") return;
    await this.db.execute(
      this.sql(
        `DELETE FROM ${this.table}
          WHERE principal_kind = ? AND principal_id = ?
            AND resource_kind = ? AND resource_id = ?`,
      ),
      [principal.kind, principal.id, resource.kind, resourceId(resource)],
    );
  }
}

// ── In-memory implementation ─────────────────────────────────────────────────

/**
 * Grant storage in a `Map` — for tests, local development, and any product
 * whose grants are seeded rather than edited.
 *
 * A real no-op, not a stub: it implements the same semantics the SQL store
 * does, including one-role-per-(principal, resource) replacement on `grant`.
 */
export class MemoryGrantStore implements GrantStore {
  private readonly rows = new Map<string, Grant>();

  constructor(seed: readonly Grant[] = []) {
    for (const g of seed) this.rows.set(MemoryGrantStore.key(g.principal, g.resource), g);
  }

  private static key(principal: Principal, resource: Resource): string {
    const p = principal.kind === "anonymous" ? "anonymous" : `${principal.kind}:${principal.id}`;
    return `${p}|${resource.kind}:${resourceId(resource)}`;
  }

  async grantsFor(principal: Principal): Promise<Grant[]> {
    if (principal.kind === "anonymous") return [];
    return [...this.rows.values()].filter(
      (g) =>
        g.principal.kind === principal.kind &&
        (g.principal as { id?: string }).id === principal.id,
    );
  }

  async grantsOn(resource: Resource): Promise<Grant[]> {
    return [...this.rows.values()].filter(
      (g) =>
        g.resource.kind === resource.kind &&
        resourceId(g.resource) === resourceId(resource),
    );
  }

  async grant(principal: Principal, resource: Resource, role: Role): Promise<void> {
    if (principal.kind === "anonymous") {
      throw new Error("MemoryGrantStore.grant: anonymous can hold no grant");
    }
    this.rows.set(MemoryGrantStore.key(principal, resource), {
      principal,
      resource,
      role,
    });
  }

  async revoke(principal: Principal, resource: Resource): Promise<void> {
    this.rows.delete(MemoryGrantStore.key(principal, resource));
  }
}
