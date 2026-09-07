export type QueuedAction<T> = {
  readonly id: string
  readonly payload: T
  readonly enqueuedAt: Date
  retries: number
}

export type QueueOptions = {
  /** Maximum number of retry attempts before an action is abandoned. Default: 3. */
  maxRetries?: number
}

/**
 * Typed offline queue — accumulates actions when connectivity is unavailable
 * and replays them in order on reconnect.
 *
 * Key invariant (from Ring of Pursuit): actions replay at their original
 * `enqueuedAt` timestamp, not at sync time, so event ordering stays coherent.
 */
export class OfflineQueue<T> {
  private readonly items: QueuedAction<T>[] = []
  private readonly maxRetries: number

  constructor(options: QueueOptions = {}) {
    this.maxRetries = options.maxRetries ?? 3
  }

  enqueue(id: string, payload: T): QueuedAction<T> {
    const action: QueuedAction<T> = { id, payload, enqueuedAt: new Date(), retries: 0 }
    this.items.push(action)
    return action
  }

  dequeue(): QueuedAction<T> | undefined {
    return this.items.shift()
  }

  /** Remove and return all queued actions for batch replay. */
  drain(): QueuedAction<T>[] {
    return this.items.splice(0, this.items.length)
  }

  /**
   * Re-queue a failed action at the front of the queue.
   * Returns false if the action has exceeded `maxRetries`.
   */
  retry(action: QueuedAction<T>): boolean {
    if (action.retries >= this.maxRetries) return false
    action.retries += 1
    this.items.unshift(action)
    return true
  }

  get size(): number {
    return this.items.length
  }

  get isEmpty(): boolean {
    return this.items.length === 0
  }
}
