/**
 * Typed offline queue — accumulates actions when connectivity is unavailable
 * and replays them in order on reconnect.
 *
 * Harvested from Ring of Pursuit in two passes, which is worth recording
 * because the first pass was incomplete in a way that mattered.
 *
 * **Pass one** took the invariant: actions replay at their original
 * `enqueuedAt` timestamp, not at sync time, so event ordering stays coherent
 * across a reconnect.
 *
 * **Pass two** (Phase 20) took the durability. An in-memory queue loses its
 * contents on a page reload — and a page reload is exactly what an offline
 * user does. Storage is now an injected adapter so the durable path and the
 * test path are the same code.
 *
 * See `docs/harvest/ring-of-pursuit.md`.
 */

export type QueuedAction<T> = {
  readonly id: string
  readonly payload: T
  readonly enqueuedAt: Date
  retries: number
}

/**
 * Where a queue keeps its actions.
 *
 * The in-memory implementation is the default and is what tests use. A browser
 * product supplies an IndexedDB-backed one so the queue survives the reload
 * that being offline tends to cause.
 *
 * Deliberately synchronous: an async storage interface would make `enqueue`
 * async, and an `enqueue` you can forget to await is a lost action. Durable
 * adapters mirror into their backing store and reconcile on construction.
 */
export type QueueStorage<T> = {
  readonly all: () => QueuedAction<T>[]
  readonly push: (action: QueuedAction<T>) => void
  readonly unshift: (action: QueuedAction<T>) => void
  readonly shift: () => QueuedAction<T> | undefined
  readonly clear: () => void
}

/**
 * The default backoff schedule: roughly 31 seconds of total delay.
 *
 * Harvested verbatim from Ring of Pursuit, where it was tied to a stated
 * decision: retry for roughly 30–60 seconds, then surface the failure rather
 * than retrying indefinitely. The number is short on purpose — a queue that
 * retries for minutes hides a real outage behind a spinner.
 *
 * Exported because it is a declared fact, not a magic number: a product that
 * needs a different envelope should override it explicitly and say why.
 */
export const DEFAULT_BACKOFF_SCHEDULE_MS: readonly number[] = [1000, 2000, 4000, 8000, 16000]

export type QueueOptions<T> = {
  /**
   * Maximum retry attempts before an action is abandoned.
   * Defaults to the length of `backoffScheduleMs`, so the count and the
   * schedule cannot disagree — a retry count without a matching schedule was
   * the gap in the first version of this queue.
   */
  maxRetries?: number
  /** Delay before each successive retry. Defaults to {@link DEFAULT_BACKOFF_SCHEDULE_MS}. */
  backoffScheduleMs?: readonly number[]
  /** Where actions live. Defaults to memory. */
  storage?: QueueStorage<T>
}

/** In-memory storage — the default, and what tests use. */
export function memoryStorage<T>(): QueueStorage<T> {
  const items: QueuedAction<T>[] = []
  return {
    all: () => [...items],
    push: (action) => {
      items.push(action)
    },
    unshift: (action) => {
      items.unshift(action)
    },
    shift: () => items.shift(),
    clear: () => {
      items.length = 0
    },
  }
}

export class OfflineQueue<T> {
  private readonly storage: QueueStorage<T>
  private readonly maxRetries: number
  private readonly backoffScheduleMs: readonly number[]

  constructor(options: QueueOptions<T> = {}) {
    this.storage = options.storage ?? memoryStorage<T>()
    this.backoffScheduleMs = options.backoffScheduleMs ?? DEFAULT_BACKOFF_SCHEDULE_MS
    this.maxRetries = options.maxRetries ?? this.backoffScheduleMs.length
  }

  enqueue(id: string, payload: T): QueuedAction<T> {
    const action: QueuedAction<T> = { id, payload, enqueuedAt: new Date(), retries: 0 }
    this.storage.push(action)
    return action
  }

  dequeue(): QueuedAction<T> | undefined {
    return this.storage.shift()
  }

  /** Remove and return all queued actions for batch replay, oldest first. */
  drain(): QueuedAction<T>[] {
    const items = this.storage.all()
    this.storage.clear()
    return items
  }

  /**
   * Re-queue a failed action at the front of the queue.
   * Returns false if the action has exceeded `maxRetries`.
   */
  retry(action: QueuedAction<T>): boolean {
    if (action.retries >= this.maxRetries) return false
    action.retries += 1
    this.storage.unshift(action)
    return true
  }

  /**
   * How long to wait before attempting an action again, in milliseconds.
   *
   * Returns `undefined` when the action has exhausted its retries — the caller
   * should surface the failure rather than wait. The last delay in the schedule
   * repeats if `maxRetries` exceeds the schedule length.
   */
  backoffFor(action: QueuedAction<T>): number | undefined {
    if (action.retries >= this.maxRetries) return undefined
    const index = Math.min(action.retries, this.backoffScheduleMs.length - 1)
    return this.backoffScheduleMs[index]
  }

  /**
   * Actions still queued, without removing them.
   *
   * Ring of Pursuit's `countPendingActions` existed to drive an "N pending
   * sync" indicator; `pending().length` covers that, and returning the actions
   * lets a caller filter by payload as well as count.
   */
  pending(): QueuedAction<T>[] {
    return this.storage.all()
  }

  get size(): number {
    return this.storage.all().length
  }

  get isEmpty(): boolean {
    return this.size === 0
  }
}
