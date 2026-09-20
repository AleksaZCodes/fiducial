/**
 * System One contract — fast, typed, probabilistic decisions.
 *
 * A System One model answers *typed questions about a state*. It does not
 * generate text: you send a state and a map of named questions, and every
 * question comes back as a typed value with a probability distribution, all
 * evaluated in parallel in one round-trip. TypeSafe's Jev is the first such
 * model (70–500ms, $0.042/M input, output free).
 *
 * Three question types, each with a differently-shaped answer:
 *
 * | Type   | Asks                    | Returns                                          |
 * |--------|-------------------------|--------------------------------------------------|
 * | Noul   | Is this true?           | `noul` (0–1). **No `confidence`** — see below.    |
 * | Choice | Which of these options? | `choice`, `probabilities`, `confidence`          |
 * | Score  | Which level?            | `score`, `legend`, `probabilities`, `confidence` |
 *
 * # Why a separate contract from `Ai`
 *
 * `Ai` takes a turn history and returns text or tool calls. `SystemOne` takes
 * a flat state and named question schemas and returns typed answers with
 * probability distributions. The two have no shared surface, and the
 * difference is not cosmetic: coercing decisions through a chat-completions
 * shape is exactly the mismatch a System One model exists to remove. A shared
 * contract would pick the wrong shape for one of them.
 *
 * # Two vendors, one wire shape
 *
 * `openrouter` and `typesafe` speak the *same* request and response bodies —
 * OpenRouter proxies to `api.typesafe.ai` through a dedicated Decisions
 * router rather than squeezing it through chat completions, so nothing is
 * lost by going through the gateway. They differ in three things: the URL,
 * which secret authenticates, and whether the model id carries a
 * `typesafe/` prefix. OpenRouter additionally returns `usage.cost` and
 * accepts `sessionId` for grouping.
 *
 * `openrouter` is the default because a product that already declared
 * `ai = "openrouter"` has the key for it, and one key covering both
 * contracts is one secret to rotate rather than two.
 *
 * # Ask one narrow judgment per question
 *
 * A question should be a judgment a knowledgeable person makes in a second.
 * "Does this convey urgency?" is a question; "analyze this and decide what to
 * do" is not — decompose it and combine the answers in code. Questions in one
 * request are evaluated independently and cannot see each other's answers, so
 * adding a question costs almost no latency and never creates context-rot.
 * Chain instead of batching only when one answer is genuinely input to
 * another, by putting the first answer into the second request's state.
 *
 * Reference a specific field of an object state from `instructions` by naming
 * its key in backticks.
 */

// ── Question types ────────────────────────────────────────────────────────────

/**
 * The question text, or structure carrying it.
 *
 * A string suffices for most questions. An object or array lets a long
 * question hold its data in separate fields — put the question in one field,
 * the data it refers to in others, and reference those fields by name in
 * backticks. Same for a `criteria` description.
 */
export type Instructions = string | Record<string, unknown> | unknown[];

/**
 * A yes/no question. Returns the probability the answer is yes.
 *
 * Use Noul when the probability itself is the signal. Do **not** use it to
 * measure intensity: a `noul` of 0.5 means the model gives yes and no equal
 * probability, not "medium". "Is this candidate strong in Python?" is a bad
 * Noul because "strong" is undefined — either define the condition sharply
 * ("does the resume state they used Python at work?") or use a Score.
 */
export interface NoulQuestion {
  type: "noul";
  instructions: Instructions;
  /**
   * Optional clarification of what yes and no mean.
   *
   * Optional per the API — a well-phrased question often needs no gloss. Add
   * it when "yes" is ambiguous.
   */
  criteria?: { true?: Instructions; false?: Instructions };
}

/**
 * Pick one option from a set you define. Returns the chosen key and the full
 * distribution.
 *
 * Use Choice when options have no order between them. Include an `other` or
 * `none_of_the_above` option whenever the list might not cover every input —
 * the model can only answer within the options given, so a missing escape
 * hatch forces a wrong answer rather than an uncertain one.
 */
export interface ChoiceQuestion {
  type: "choice";
  instructions: Instructions;
  /**
   * Option key → description of that option. `null` when the key needs no
   * gloss. Maximum 255 options.
   */
  criteria: Record<string, Instructions | null>;
}

/**
 * Rate the state along an ordered rubric. Returns a probability-weighted
 * position that can land between levels.
 *
 * Use Score when the answer falls on a spectrum you can describe.
 */
export interface ScoreQuestion {
  type: "score";
  instructions: Instructions;
  /** Ordered level descriptions, lowest first. At least 2, at most 10. */
  criteria: Instructions[];
}

export type Question = NoulQuestion | ChoiceQuestion | ScoreQuestion;

// ── Answer types ──────────────────────────────────────────────────────────────

/**
 * The probability that the answer is yes, from 0 (no) to 1 (yes).
 *
 * Near 0.5 means genuine uncertainty. Noul carries **no** `confidence` field:
 * the value is already the distribution over two outcomes, so a separate
 * concentration measure would restate it.
 */
export interface NoulAnswer {
  type: "noul";
  noul: number;
}

export interface ChoiceAnswer {
  type: "choice";
  /** The highest-probability option key. */
  choice: string;
  /** Every option mapped to its probability. Sums to 1. */
  probabilities: Record<string, number>;
  /** How peaked `probabilities` is, 0–1. Derived from it, not a second signal. */
  confidence: number;
}

export interface ScoreAnswer {
  type: "score";
  /**
   * Probability-weighted position across the levels — can land between two
   * of them (e.g. 1.05 is just above level 1).
   */
  score: number;
  /** Level index (as a string key) → its description. */
  legend: Record<string, string>;
  /** Level index (as a string key) → its probability. Sums to 1. */
  probabilities: Record<string, number>;
  confidence: number;
}

export type Answer = NoulAnswer | ChoiceAnswer | ScoreAnswer;

// ── Request / response ────────────────────────────────────────────────────────

export interface SystemOneRequest {
  /**
   * The content to evaluate: a plain string, or an object/array of related
   * context. Every question in this request sees this same state.
   */
  state: string | Record<string, unknown> | unknown[];
  /** Named questions. Each key names its answer in the response. */
  questions: Record<string, Question>;
  /** Override the declared model for this one call. */
  model?: string;
  /**
   * Group related requests for observability (OpenRouter only, ≤256 chars).
   *
   * Never sent to the model, so it cannot affect an answer. Useful for tying
   * every question asked during one run together in the gateway's logs.
   */
  sessionId?: string;
}

export interface SystemOneUsage {
  inputTokens: number;
  outputTokens: number;
  /** Billed cost in USD. OpenRouter reports it; the direct API does not. */
  cost?: number;
}

export interface SystemOneResponse {
  /** Answers under the same keys the request used for `questions`. */
  answers: Record<string, Answer>;
  usage: SystemOneUsage;
  /** The model that actually served the call, e.g. `typesafe/jev-1.13-20260917`. */
  model: string;
  /** Gateway request id, when the vendor returns one. */
  id?: string;
  /** Upstream provider name, when the vendor reports one. */
  provider?: string;
}

// ── Error ─────────────────────────────────────────────────────────────────────

export type SystemOneErrorKind =
  /** Malformed request or a question that failed validation (400, 422). */
  | "invalid_request"
  /** Missing or bad key (401, 403). */
  | "unauthorized"
  /** Out of credits (402) — OpenRouter only. */
  | "insufficient_credits"
  | "rate_limited"
  /** Vendor temporarily overloaded (529) or a 5xx. */
  | "overloaded"
  | "other";

export class SystemOneError extends Error {
  constructor(
    message: string,
    public readonly kind: SystemOneErrorKind = "other",
    /** Seconds the vendor asked us to wait, when it said. */
    public readonly retryAfterSeconds?: number,
    /** HTTP status, when the failure was a response rather than a transport error. */
    public readonly status?: number,
  ) {
    super(message);
    this.name = "SystemOneError";
  }
}

// ── Contract ──────────────────────────────────────────────────────────────────

/** Fast, typed, probabilistic decisions about a state. */
export interface SystemOne {
  decide(request: SystemOneRequest): Promise<SystemOneResponse>;
}

// ── Answer helpers ────────────────────────────────────────────────────────────

/**
 * Narrow an `Answer` to a `Noul` and return its probability.
 *
 * These three exist because `answers` is keyed by strings the caller chose,
 * so TypeScript cannot know which variant came back under which key. Without
 * them every call site writes the same cast, and a cast is exactly where a
 * mixed-up question id stops being a caught error. Each throws naming the key
 * and what actually arrived.
 */
export function noulOf(answers: Record<string, Answer>, key: string): number {
  const a = expect(answers, key, "noul");
  return (a as NoulAnswer).noul;
}

export function choiceOf(answers: Record<string, Answer>, key: string): ChoiceAnswer {
  return expect(answers, key, "choice") as ChoiceAnswer;
}

export function scoreOf(answers: Record<string, Answer>, key: string): ScoreAnswer {
  return expect(answers, key, "score") as ScoreAnswer;
}

function expect(answers: Record<string, Answer>, key: string, type: Answer["type"]): Answer {
  const a = answers[key];
  if (!a) {
    throw new SystemOneError(
      `no answer under \`${key}\` — got [${Object.keys(answers).join(", ")}]`,
      "other",
    );
  }
  if (a.type !== type) {
    throw new SystemOneError(
      `answer \`${key}\` is a ${a.type}, expected a ${type}`,
      "other",
    );
  }
  return a;
}

// ── None implementation ───────────────────────────────────────────────────────

const NONE_MESSAGE =
  'systemOne = "none": no System One vendor is selected. Set ' +
  '`[adapters] systemOne = "openrouter"` in fiducial.toml, then re-run ' +
  "`fid derive`.";

/**
 * No-op System One — every call throws `SystemOneError` naming the fix.
 *
 * Fails rather than returning an empty answer, for the same reason as
 * `NoneAi`: a decision *is* the result the caller reads, so a fabricated one
 * turns "no vendor configured" into a confidently wrong branch taken in
 * production. Constructing it stays free, so it sits in `AdapterSet` from the
 * first commit and nothing fails until something actually asks.
 */
export class NoneSystemOne implements SystemOne {
  constructor(_env?: unknown, _model?: string) {}

  async decide(_request: SystemOneRequest): Promise<SystemOneResponse> {
    throw new SystemOneError(NONE_MESSAGE, "unauthorized");
  }
}

// ── Shared vendor machinery ───────────────────────────────────────────────────

/** Tuning for the retry loop. Defaults follow the vendors' own guidance. */
export interface RetryPolicy {
  /** Total attempts including the first. Default 3. */
  attempts?: number;
  /** First backoff step in ms; doubles each retry. Default 250. */
  baseDelayMs?: number;
  /** Cap on any single backoff. Default 8000. */
  maxDelayMs?: number;
}

function classify(status: number): SystemOneErrorKind {
  if (status === 401 || status === 403) return "unauthorized";
  if (status === 402) return "insufficient_credits";
  if (status === 400 || status === 422 || status === 413) return "invalid_request";
  if (status === 429) return "rate_limited";
  if (status === 529 || status >= 500) return "overloaded";
  return "other";
}

/** Only a transient failure is worth a second attempt. */
function retryable(kind: SystemOneErrorKind): boolean {
  return kind === "rate_limited" || kind === "overloaded";
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

interface WireResponse {
  model?: string;
  id?: string;
  provider?: string;
  answers?: Record<string, Answer>;
  usage?: { input_tokens?: number; output_tokens?: number; cost?: number };
  error?: { message?: string; code?: number | string };
}

/**
 * POST one decisions request, retrying transient failures with exponential
 * backoff.
 *
 * Both vendors' docs require backoff on 429 and 529 and ship SDKs that do it
 * by default. This contract takes no vendor SDK — same policy as `ai.ts`, and
 * the wire shape here is small enough that an SDK would buy only this loop —
 * so the loop is written once and shared by both vendors.
 *
 * `Retry-After` wins over the computed backoff when the vendor sends it:
 * it is the only party that knows when capacity returns. Jitter is added to
 * the computed delay so a fan-out of parallel callers does not retry in
 * lockstep and reproduce the burst that triggered the limit.
 */
async function post(
  url: string,
  apiKey: string,
  body: Record<string, unknown>,
  fetchImpl: typeof fetch,
  vendor: string,
  policy: RetryPolicy,
): Promise<WireResponse> {
  const attempts = Math.max(1, policy.attempts ?? 3);
  const base = policy.baseDelayMs ?? 250;
  const cap = policy.maxDelayMs ?? 8000;

  let last: SystemOneError | undefined;

  for (let attempt = 1; attempt <= attempts; attempt++) {
    let res: Response;
    try {
      res = await fetchImpl(url, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(body),
      });
    } catch (err) {
      // A transport failure has no status; treat it as transient — a DNS blip
      // or a dropped connection is exactly what a retry is for.
      last = new SystemOneError(`${vendor}: request failed: ${String(err)}`, "overloaded");
      if (attempt < attempts) {
        await sleep(Math.min(cap, base * 2 ** (attempt - 1)) * (0.5 + Math.random()));
        continue;
      }
      throw last;
    }

    if (res.ok) {
      const data = (await res.json()) as WireResponse;
      // A 200 carrying `error` is a real gateway shape: an upstream provider
      // failing mid-request is reported in the body of a successful response.
      if (data.error?.message && !data.answers) {
        throw new SystemOneError(`${vendor}: ${data.error.message}`, "other", undefined, 200);
      }
      return data;
    }

    const text = await res.text().catch(() => "");
    let message = text;
    try {
      const parsed = JSON.parse(text) as WireResponse;
      if (parsed.error?.message) message = parsed.error.message;
    } catch {
      // Not JSON — the raw body is the best message available.
    }

    const kind = classify(res.status);
    const header = Number(res.headers?.get?.("retry-after") ?? "");
    const retryAfter = Number.isFinite(header) && header > 0 ? header : undefined;

    last = new SystemOneError(
      `${vendor} returned HTTP ${res.status}: ${message || "(no body)"}`,
      kind,
      retryAfter,
      res.status,
    );

    if (attempt < attempts && retryable(kind)) {
      const backoff = Math.min(cap, base * 2 ** (attempt - 1)) * (0.5 + Math.random());
      await sleep(retryAfter !== undefined ? retryAfter * 1000 : backoff);
      continue;
    }
    throw last;
  }

  // Unreachable: the loop either returns or throws. Satisfies the checker
  // without inventing a different error than the one that actually happened.
  throw last ?? new SystemOneError(`${vendor}: exhausted retries`, "other");
}

function normalize(data: WireResponse, vendor: string, fallbackModel: string): SystemOneResponse {
  if (!data.answers) {
    throw new SystemOneError(
      `${vendor}: response had no answers${data.error?.message ? `: ${data.error.message}` : ""}`,
      "other",
    );
  }
  return {
    answers: data.answers,
    usage: {
      inputTokens: data.usage?.input_tokens ?? 0,
      outputTokens: data.usage?.output_tokens ?? 0,
      ...(data.usage?.cost !== undefined ? { cost: data.usage.cost } : {}),
    },
    model: data.model ?? fallbackModel,
    ...(data.id ? { id: data.id } : {}),
    ...(data.provider ? { provider: data.provider } : {}),
  };
}

// ── OpenRouter ────────────────────────────────────────────────────────────────

const OPENROUTER_DECISIONS_URL = "https://openrouter.ai/api/alpha/decisions";

/** Default when `[system-one] model` is unset. Tracks the newest Jev. */
export const OPENROUTER_DEFAULT_MODEL = "typesafe/jev-latest";

/**
 * System One through OpenRouter's Decisions router.
 *
 * Note the path: `/api/alpha/decisions`, **not** under `/api/v1`, and not
 * chat completions. OpenRouter routes decisions models through a dedicated
 * adapter that preserves the typed-answer shape — the models are marked
 * `output_modalities: ["decisions"]` and `has_text_output: false`, which is
 * also why they do not appear in a default `/api/v1/models` listing. Query
 * `?output_modalities=decisions` to see them.
 *
 * Secret-reached: `env.OPENROUTER_API_KEY` — the same key
 * `[adapters] ai = "openrouter"` already uses. That sharing is the reason
 * this is the default vendor: one secret to set and rotate for both contracts.
 *
 * The constructor throws when the key is absent so a misconfiguration is a
 * deploy-time error rather than a first-call one.
 */
export class OpenRouterSystemOne implements SystemOne {
  private readonly apiKey: string;
  private readonly defaultModel: string;
  private readonly fetchImpl: typeof fetch;
  private readonly policy: RetryPolicy;

  constructor(
    env: { OPENROUTER_API_KEY?: string },
    model?: string,
    fetchImpl: typeof fetch = fetch,
    policy: RetryPolicy = {},
  ) {
    if (!env?.OPENROUTER_API_KEY) {
      throw new SystemOneError(
        "OpenRouterSystemOne: no `OPENROUTER_API_KEY` on env — set it with " +
          "`wrangler secret put OPENROUTER_API_KEY`",
        "unauthorized",
      );
    }
    this.apiKey = env.OPENROUTER_API_KEY;
    this.defaultModel = model ?? OPENROUTER_DEFAULT_MODEL;
    this.fetchImpl = fetchImpl;
    this.policy = policy;
  }

  async decide(request: SystemOneRequest): Promise<SystemOneResponse> {
    const model = request.model ?? this.defaultModel;
    const body: Record<string, unknown> = {
      model,
      state: request.state,
      questions: request.questions,
    };
    if (request.sessionId) body.session_id = request.sessionId;

    const data = await post(
      OPENROUTER_DECISIONS_URL,
      this.apiKey,
      body,
      this.fetchImpl,
      "OpenRouterSystemOne",
      this.policy,
    );
    return normalize(data, "OpenRouterSystemOne", model);
  }
}

// ── TypeSafe (direct) ─────────────────────────────────────────────────────────

const TYPESAFE_URL = "https://api.typesafe.ai/v1/systemone";

/** Default when `[system-one] model` is unset, on the direct API. */
export const TYPESAFE_DEFAULT_MODEL = "jev-latest";

/**
 * System One straight from TypeSafe, bypassing the gateway.
 *
 * Same request and response bodies as the OpenRouter vendor — only the URL,
 * the secret and the unprefixed model id differ. `usage.cost` is absent here
 * because TypeSafe reports tokens and leaves pricing to the caller.
 *
 * Prefer `openrouter` unless you specifically want no gateway in the path —
 * a separate billing relationship, a data-residency requirement, or an
 * early-access model the gateway has not listed yet. `sessionId` is ignored:
 * it is an OpenRouter observability field, so honoring it here would be
 * pretending it did something.
 *
 * Secret-reached: `env.TYPESAFE_API_KEY`.
 */
export class TypeSafeSystemOne implements SystemOne {
  private readonly apiKey: string;
  private readonly defaultModel: string;
  private readonly fetchImpl: typeof fetch;
  private readonly policy: RetryPolicy;

  constructor(
    env: { TYPESAFE_API_KEY?: string },
    model?: string,
    fetchImpl: typeof fetch = fetch,
    policy: RetryPolicy = {},
  ) {
    if (!env?.TYPESAFE_API_KEY) {
      throw new SystemOneError(
        "TypeSafeSystemOne: no `TYPESAFE_API_KEY` on env — set it with " +
          "`wrangler secret put TYPESAFE_API_KEY`",
        "unauthorized",
      );
    }
    this.apiKey = env.TYPESAFE_API_KEY;
    this.defaultModel = model ?? TYPESAFE_DEFAULT_MODEL;
    this.fetchImpl = fetchImpl;
    this.policy = policy;
  }

  async decide(request: SystemOneRequest): Promise<SystemOneResponse> {
    const model = request.model ?? this.defaultModel;
    const data = await post(
      TYPESAFE_URL,
      this.apiKey,
      { model, state: request.state, questions: request.questions },
      this.fetchImpl,
      "TypeSafeSystemOne",
      this.policy,
    );
    return normalize(data, "TypeSafeSystemOne", model);
  }
}
