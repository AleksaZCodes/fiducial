/**
 * Diagnostics contract — error tracking and observability.
 *
 * Maps to `errors` in `fiducial.toml [adapters]`. Named `Diagnostics` to
 * avoid the awkward `ErrorsError`. Mirrors `fiducial_adapters::diagnostics`.
 *
 * Intentionally synchronous — capture-and-send is fire-and-forget.
 */

export type DiagnosticsLevel = "debug" | "info" | "warning" | "error" | "fatal";

/** Diagnostics contract. */
export interface Diagnostics {
  /**
   * Capture an error with optional additional context string.
   *
   * Fire-and-forget: the implementation queues the event and returns
   * immediately. Do not await anything that should not block the caller.
   */
  captureError(error: unknown, context?: string): void;

  /** Capture a free-form message at the given level. */
  captureMessage(level: DiagnosticsLevel, message: string): void;
}

// ── None implementation ───────────────────────────────────────────────────────

/**
 * No-op diagnostics — all captures are silently dropped.
 *
 * `errors = "none"` is the default. Switching to Sentry or Workers Analytics
 * is a config change, not a refactor.
 */
export class NoneDiagnostics implements Diagnostics {
  // Accepts and ignores `env` so every vendor class in this contract shares
  // one constructor shape for the generated factory to call uniformly.
  constructor(_env?: unknown) {}

  captureError(_error: unknown, _context?: string): void {}
  captureMessage(_level: DiagnosticsLevel, _message: string): void {}
}
