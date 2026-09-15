/**
 * Queue contract — the producer side of an asynchronous job/message queue.
 *
 * Mirrors `fiducial_adapters::queue` in Rust for the contract and `none`.
 * `cloudflare-queues` is real only here: a Cloudflare Queue producer binding
 * exists only inside a Worker. See the Rust module's doc comment for why
 * there is no `receive`/`consume` method on this contract at all — a
 * consumer is an exported handler, not a call an adapter makes.
 */

export class QueueError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "QueueError";
  }
}

/** Asynchronous queue contract — producer side only. */
export interface Queue {
  /** Enqueue a single message body. */
  send(body: Uint8Array): Promise<void>;

  /**
   * Enqueue several message bodies as one batch.
   *
   * No atomicity promise across the batch — unlike `Database.batch`, no
   * vendor behind this contract has a real batch/transaction primitive to
   * lean on for it.
   */
  sendBatch(bodies: Uint8Array[]): Promise<void>;
}

// ── None implementation ───────────────────────────────────────────────────────

/** No-op queue — every send succeeds silently and the message is discarded. */
export class NoneQueue implements Queue {
  // Accepts and ignores `env`, matching every other vendor class in this
  // contract, for the generated factory to call uniformly.
  constructor(_env?: unknown) {}

  async send(_body: Uint8Array): Promise<void> {}

  async sendBatch(_bodies: Uint8Array[]): Promise<void> {}
}

// ── Cloudflare Queues ────────────────────────────────────────────────────────

/**
 * `CloudflareBinding` mirrors the methods this adapter calls from
 * `@cloudflare/workers-types`' `Queue`, without depending on that package at
 * the type level — same reasoning as `D1Binding`/`R2Binding` in the other
 * two contracts.
 */
interface CloudflareQueueBinding {
  send(message: unknown, options?: { contentType?: string }): Promise<void>;
  sendBatch(
    messages: Iterable<{ body: unknown; contentType?: string }>,
  ): Promise<void>;
}

/**
 * Cloudflare Queues — reached through a binding (`env.QUEUE`) inside a
 * Worker. Bytes are sent with `contentType: "bytes"` so the consumer handler
 * receives the same `Uint8Array` back rather than Cloudflare's default JSON
 * round-trip.
 *
 * **Binding convention:** `env.QUEUE`, for the same reason `D1Database`
 * reads `env.DB` — `fid derive` cannot see a hand-edited `wrangler.toml`.
 */
export class CloudflareQueue implements Queue {
  private readonly queue: CloudflareQueueBinding;

  constructor(env: { QUEUE?: CloudflareQueueBinding }) {
    if (!env?.QUEUE) {
      throw new QueueError(
        "CloudflareQueue: no `QUEUE` binding on env — add a [[queues.producers]] " +
          'block with `binding = "QUEUE"` to wrangler.toml',
      );
    }
    this.queue = env.QUEUE;
  }

  async send(body: Uint8Array): Promise<void> {
    try {
      await this.queue.send(body, { contentType: "bytes" });
    } catch (err) {
      throw new QueueError(`Cloudflare Queues send failed: ${String(err)}`);
    }
  }

  async sendBatch(bodies: Uint8Array[]): Promise<void> {
    try {
      await this.queue.sendBatch(
        bodies.map((body) => ({ body, contentType: "bytes" })),
      );
    } catch (err) {
      throw new QueueError(
        `Cloudflare Queues sendBatch failed: ${String(err)}`,
      );
    }
  }
}
