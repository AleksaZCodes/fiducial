/**
 * Contract 3: Postgres Changes
 *
 * CDC stream from a Postgres table.  Delivers INSERT, UPDATE, and DELETE events
 * with typed row payloads.
 *
 * The declared shape is the row type `R`.  Every event carries `R` (or
 * `Partial<R>` for UPDATE old values) at the TypeScript level.
 */

/** The three Postgres CDC event types. */
export type PostgresEventType = 'INSERT' | 'UPDATE' | 'DELETE';

/** Base for all Postgres change events. */
export interface PostgresEvent<R> {
  schema: string;
  table: string;
  commitTimestamp: string;
}

/** A new row was inserted. */
export interface PostgresInsert<R> extends PostgresEvent<R> {
  eventType: 'INSERT';
  new: R;
}

/** An existing row was updated. */
export interface PostgresUpdate<R> extends PostgresEvent<R> {
  eventType: 'UPDATE';
  /** The row after the update. */
  new: R;
  /** The row before the update (only fields in the replica identity). */
  old: Partial<R>;
}

/** An existing row was deleted. */
export interface PostgresDelete<R> extends PostgresEvent<R> {
  eventType: 'DELETE';
  /** The deleted row (only fields in the replica identity). */
  old: Partial<R>;
}

/** Union of all Postgres change events. */
export type AnyPostgresEvent<R> =
  | PostgresInsert<R>
  | PostgresUpdate<R>
  | PostgresDelete<R>;

/** Subscription filter that matches the Supabase API shape. */
export interface PostgresFilter {
  schema: string;
  table?: string;
  filter?: string;
}

/** Adapter required to subscribe to Postgres changes. */
export interface PostgresChangesAdapter<R> {
  /**
   * Subscribe to all three event types on one filter.
   * Returns an unsubscribe function.
   */
  on(
    filter: PostgresFilter,
    handler: (event: AnyPostgresEvent<R>) => void
  ): () => void;

  /** Subscribe to INSERT only. */
  onInsert(
    filter: PostgresFilter,
    handler: (event: PostgresInsert<R>) => void
  ): () => void;

  /** Subscribe to UPDATE only. */
  onUpdate(
    filter: PostgresFilter,
    handler: (event: PostgresUpdate<R>) => void
  ): () => void;

  /** Subscribe to DELETE only. */
  onDelete(
    filter: PostgresFilter,
    handler: (event: PostgresDelete<R>) => void
  ): () => void;
}

/**
 * Create a typed Postgres Changes client from an adapter.
 *
 * @example
 * ```ts
 * type Round = { id: string; state: 'pending' | 'active' | 'ended' };
 * const pg = createPostgresChanges<Round>(myAdapter);
 * pg.onInsert({ schema: 'public', table: 'rounds' }, e => console.log(e.new));
 * ```
 */
export function createPostgresChanges<R>(
  adapter: PostgresChangesAdapter<R>
): PostgresChangesAdapter<R> {
  return adapter;
}
