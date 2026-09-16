/**
 * AI contract — conversational and agentic language-model calls.
 *
 * Mirrors `fiducial_adapters::ai` in Rust, which carries the full argument for
 * why this contract targets an **AI gateway** rather than one adapter per
 * model vendor. The short version: no honest intersection of the Anthropic and
 * OpenAI shapes exists, so the contract targets one shape — the gateway's —
 * and model choice becomes a string in a declaration.
 *
 * `openrouter` is real only here, the same boundary as `Turnstile` and
 * `SupabaseAuth`: a plain HTTPS API Rust could reach, with no Rust consumer
 * that would.
 */

/** Who authored a message in the conversation. */
export type Role = "system" | "user" | "assistant" | "tool";

/** A tool invocation the model asked for. */
export interface ToolCall {
  /** Gateway-assigned id. A `tool` message answers it by id, not by name — a
   * model may call the same tool twice in one turn. */
  id: string;
  name: string;
  /**
   * Arguments as a JSON **string**, not a parsed value.
   *
   * The caller has to validate these against the schema it declared either
   * way: a model can emit arguments that are valid JSON and still wrong. An
   * adapter that parsed them would be doing half of a job the caller cannot
   * skip, and would turn a model mistake into a thrown adapter error.
   */
  arguments: string;
}

/** One turn in the conversation. */
export interface Message {
  role: Role;
  /** Text content. Empty for an assistant turn that was only tool calls. */
  content: string;
  /** Set on a `tool` message: which call this answers. */
  toolCallId?: string;
  /** Set on an `assistant` turn replayed into a follow-up request. */
  toolCalls?: ToolCall[];
}

/** A tool the model may call. */
export interface ToolDefinition {
  name: string;
  /** What the tool does. The model reads this to decide whether to call it,
   * so it is part of the prompt, not documentation. */
  description: string;
  /** JSON Schema for the arguments. */
  parameters: Record<string, unknown>;
}

/** Whether the model must call a tool. Defaults to `"auto"` when tools are present. */
export type ToolChoice = "auto" | "none" | "required";

/** A request for a completion. */
export interface ChatRequest {
  messages: Message[];
  /**
   * System prompt, top-level.
   *
   * Top-level rather than a first `system` message because the two directions
   * are not equally easy: this lowers to a system message in one line, and
   * recovering "which of these was the system prompt" from a list does not
   * always work.
   */
  system?: string;
  tools?: ToolDefinition[];
  toolChoice?: ToolChoice;
  /**
   * Override the model for this one call.
   *
   * Normally omitted: the model is a declaration (`[ai] model` in
   * `fiducial.toml`) derived into the generated factory. This exists because
   * an agent that classifies with a small model and answers with a large one
   * is a normal agent — and making that product construct a second adapter
   * would turn the declaration into a lie rather than a default.
   */
  model?: string;
  maxTokens?: number;
  temperature?: number;
}

/** Why generation stopped. */
export type StopReason = "stop" | "length" | "tool_calls" | "other";

/** Tokens billed for one call. */
export interface Usage {
  inputTokens: number;
  outputTokens: number;
}

/** A completed response. */
export interface ChatResponse {
  text: string;
  /** Non-empty exactly when `stopReason` is `"tool_calls"`. */
  toolCalls: ToolCall[];
  stopReason: StopReason;
  usage: Usage;
  /**
   * The model that actually served the call.
   *
   * Reported rather than assumed: a gateway may route to a different model
   * than the one requested, and a product logging what it *asked for* is
   * logging a guess.
   */
  model: string;
}

/** One event in a streamed response. */
export type StreamEvent =
  /** A chunk of assistant text. Concatenated in order, these are `ChatResponse.text`. */
  | { type: "text"; text: string }
  /**
   * A complete tool call.
   *
   * Emitted whole, not as argument fragments: gateways stream tool arguments
   * as partial JSON, and partial JSON is not something a consumer can do
   * anything with except buffer it. The adapter buffers once so every
   * consumer does not.
   */
  | { type: "tool_call"; call: ToolCall }
  /** The final event. Always emitted on a successful stream. */
  | { type: "done"; stopReason: StopReason; usage: Usage };

/** What went wrong, in a form a caller can branch on. */
export type AiErrorKind =
  | "rejected"
  | "rate_limited"
  | "context_length_exceeded"
  | "model_unavailable"
  | "other";

/**
 * One error class with a `kind`, mirroring the Rust `AiError` enum.
 *
 * `context_length_exceeded` and `rate_limited` are distinguished from
 * `rejected` because their remedies are programmable — drop old turns and
 * retry; back off and retry — where the remedy for `rejected` is to read the
 * message.
 */
export class AiError extends Error {
  constructor(
    message: string,
    public readonly kind: AiErrorKind = "other",
    /** Set when `kind` is `"rate_limited"`, if the gateway said. */
    public readonly retryAfterSeconds?: number,
    /** Set when the error names a specific model. */
    public readonly model?: string,
  ) {
    super(message);
    this.name = "AiError";
  }
}

/** Conversational and agentic language-model calls, through a gateway. */
export interface Ai {
  /** Run a completion to the end and return it. */
  chat(request: ChatRequest): Promise<ChatResponse>;

  /**
   * Run a completion, yielding events as they arrive.
   *
   * A separate method rather than a `stream: boolean` on `ChatRequest`,
   * because a flag that changes what comes back is a return type in disguise.
   */
  stream(request: ChatRequest): AsyncIterable<StreamEvent>;
}

// ── None implementation ───────────────────────────────────────────────────────

const NONE_MESSAGE =
  'ai = "none": no AI vendor is selected. Set `[adapters] ai = "openrouter"` ' +
  "and `[ai] model` in fiducial.toml, then re-run `fid derive`.";

/**
 * No-op AI — every call throws `AiError` naming the fix.
 *
 * **The second `None*` that fails rather than succeeding**, after `NoneAuth`.
 * A no-op send or enqueue is indistinguishable from the real thing at the call
 * site: the caller wanted an effect elsewhere and does not read a result. A
 * completion *is* the result, so returning an empty one turns "no AI vendor is
 * selected" into a blank answer in the product's UI — which reads as a model
 * bug and gets debugged as one.
 *
 * It still costs nothing to wire in: constructing it is free and it sits in
 * `AdapterSet` from the first commit. Nothing fails until something actually
 * asks for a completion, at which point failing is the honest answer.
 */
export class NoneAi implements Ai {
  // Accepts and ignores `env` and the derived model so every vendor class in
  // this contract shares one constructor shape for the generated factory.
  constructor(_env?: unknown, _model?: string) {}

  async chat(_request: ChatRequest): Promise<ChatResponse> {
    throw new AiError(NONE_MESSAGE, "rejected");
  }

  async *stream(_request: ChatRequest): AsyncIterable<StreamEvent> {
    throw new AiError(NONE_MESSAGE, "rejected");
  }
}

// ── OpenRouter ────────────────────────────────────────────────────────────────

const OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions";

/** The gateway's wire shape. Named here rather than imported — this package
 * depends on no vendor SDK, same as every other contract in it. */
interface WireToolCall {
  index?: number;
  id?: string;
  function?: { name?: string; arguments?: string };
}

interface WireChoice {
  message?: { content?: string | null; tool_calls?: WireToolCall[] };
  delta?: { content?: string | null; tool_calls?: WireToolCall[] };
  finish_reason?: string | null;
}

interface WireResponse {
  model?: string;
  choices?: WireChoice[];
  usage?: { prompt_tokens?: number; completion_tokens?: number };
  error?: { message?: string; code?: number | string };
}

function stopReasonFrom(finish: string | null | undefined): StopReason {
  switch (finish) {
    case "stop":
      return "stop";
    case "length":
      return "length";
    case "tool_calls":
    case "function_call":
      return "tool_calls";
    default:
      return "other";
  }
}

/**
 * Classify a failed response body into an `AiErrorKind`.
 *
 * Matched on the message text because the gateway multiplexes many upstream
 * providers and does not normalize these into distinct codes — a context
 * overflow arrives as a 400 whose message says so. Matching text is fragile,
 * so an unrecognized failure stays `"rejected"` (which every caller already
 * has to handle) rather than being guessed into a kind whose remedy would be
 * wrong.
 */
function classify(status: number, message: string): AiErrorKind {
  const m = message.toLowerCase();
  if (status === 429) return "rate_limited";
  if (m.includes("context length") || m.includes("maximum context")) {
    return "context_length_exceeded";
  }
  if (status === 404 || m.includes("no allowed providers") || m.includes("not a valid model")) {
    return "model_unavailable";
  }
  return "rejected";
}

/**
 * OpenRouter — a vendor-independent AI gateway speaking one normalized shape
 * across model providers.
 *
 * Secret-reached: `env.OPENROUTER_API_KEY`, set with `wrangler secret put`.
 * The constructor throws when the key is absent, with that command in the
 * message — a misconfigured product fails at construction rather than at the
 * first completion, which is the difference between a deploy-time error and a
 * user-facing one.
 *
 * `model` is the default the product declared in `[ai] model`; `fid derive`
 * passes it in. A per-call `request.model` wins over it.
 *
 * `fetchImpl` defaults to global `fetch` and exists so tests can pass a stub
 * rather than monkeypatching `globalThis.fetch`.
 */
export class OpenRouterAi implements Ai {
  private readonly apiKey: string;
  private readonly defaultModel: string;
  private readonly fetchImpl: typeof fetch;

  constructor(
    env: { OPENROUTER_API_KEY?: string },
    model?: string,
    fetchImpl: typeof fetch = fetch,
  ) {
    if (!env?.OPENROUTER_API_KEY) {
      throw new AiError(
        "OpenRouterAi: no `OPENROUTER_API_KEY` on env — set it with " +
          "`wrangler secret put OPENROUTER_API_KEY`",
        "rejected",
      );
    }
    if (!model) {
      throw new AiError(
        "OpenRouterAi: no model. Declare `[ai] model` in fiducial.toml and " +
          "re-run `fid derive`, or pass `model` on the request.",
        "model_unavailable",
      );
    }
    this.apiKey = env.OPENROUTER_API_KEY;
    this.defaultModel = model;
    this.fetchImpl = fetchImpl;
  }

  /** Translate the contract's request into the gateway's wire shape. */
  private body(request: ChatRequest, stream: boolean): Record<string, unknown> {
    const messages: Record<string, unknown>[] = [];
    if (request.system) {
      messages.push({ role: "system", content: request.system });
    }
    for (const m of request.messages) {
      const wire: Record<string, unknown> = { role: m.role, content: m.content };
      if (m.toolCallId) wire.tool_call_id = m.toolCallId;
      if (m.toolCalls?.length) {
        wire.tool_calls = m.toolCalls.map((c) => ({
          id: c.id,
          type: "function",
          function: { name: c.name, arguments: c.arguments },
        }));
      }
      messages.push(wire);
    }

    const body: Record<string, unknown> = {
      model: request.model ?? this.defaultModel,
      messages,
    };
    if (stream) body.stream = true;
    if (request.tools?.length) {
      body.tools = request.tools.map((t) => ({
        type: "function",
        function: {
          name: t.name,
          description: t.description,
          parameters: t.parameters,
        },
      }));
      body.tool_choice = request.toolChoice ?? "auto";
    }
    if (request.maxTokens !== undefined) body.max_tokens = request.maxTokens;
    if (request.temperature !== undefined) body.temperature = request.temperature;
    return body;
  }

  private async post(request: ChatRequest, stream: boolean): Promise<Response> {
    let res: Response;
    try {
      res = await this.fetchImpl(OPENROUTER_URL, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(this.body(request, stream)),
      });
    } catch (err) {
      throw new AiError(`OpenRouterAi request failed: ${String(err)}`, "other");
    }

    if (!res.ok) {
      const text = await res.text().catch(() => "");
      let message = text;
      try {
        const parsed = JSON.parse(text) as WireResponse;
        if (parsed.error?.message) message = parsed.error.message;
      } catch {
        // Not JSON — the raw body is the best message available.
      }
      const kind = classify(res.status, message);
      const retryAfter = Number(res.headers?.get?.("retry-after") ?? "");
      throw new AiError(
        `OpenRouterAi returned HTTP ${res.status}: ${message || "(no body)"}`,
        kind,
        Number.isFinite(retryAfter) && retryAfter > 0 ? retryAfter : undefined,
        request.model ?? this.defaultModel,
      );
    }

    return res;
  }

  async chat(request: ChatRequest): Promise<ChatResponse> {
    const res = await this.post(request, false);
    const data = (await res.json()) as WireResponse;
    const choice = data.choices?.[0];

    // A 200 carrying an `error` is a real OpenRouter shape, not defensive
    // coding: an upstream provider failing mid-request is reported in the body
    // of an otherwise successful response.
    if (!choice) {
      throw new AiError(
        `OpenRouterAi: response had no choices${
          data.error?.message ? `: ${data.error.message}` : ""
        }`,
        classify(200, data.error?.message ?? ""),
      );
    }

    const toolCalls: ToolCall[] = (choice.message?.tool_calls ?? []).map((c) => ({
      id: c.id ?? "",
      name: c.function?.name ?? "",
      arguments: c.function?.arguments ?? "",
    }));

    return {
      text: choice.message?.content ?? "",
      toolCalls,
      stopReason: stopReasonFrom(choice.finish_reason),
      usage: {
        inputTokens: data.usage?.prompt_tokens ?? 0,
        outputTokens: data.usage?.completion_tokens ?? 0,
      },
      model: data.model ?? request.model ?? this.defaultModel,
    };
  }

  async *stream(request: ChatRequest): AsyncIterable<StreamEvent> {
    const res = await this.post(request, true);
    if (!res.body) {
      throw new AiError("OpenRouterAi: streamed response had no body", "other");
    }

    // Tool calls arrive as argument fragments keyed by `index`. Accumulate
    // them and emit each whole, once — see `StreamEvent`'s doc for why.
    const pending = new Map<number, { id: string; name: string; args: string }>();
    let stopReason: StopReason = "other";
    let usage: Usage = { inputTokens: 0, outputTokens: 0 };

    for await (const data of sseData(res.body)) {
      let parsed: WireResponse;
      try {
        parsed = JSON.parse(data) as WireResponse;
      } catch {
        // OpenRouter interleaves non-JSON keep-alive payloads; skipping one is
        // correct, and `[DONE]` never reaches here.
        continue;
      }

      if (parsed.error?.message) {
        throw new AiError(
          `OpenRouterAi stream error: ${parsed.error.message}`,
          classify(200, parsed.error.message),
        );
      }
      if (parsed.usage) {
        usage = {
          inputTokens: parsed.usage.prompt_tokens ?? 0,
          outputTokens: parsed.usage.completion_tokens ?? 0,
        };
      }

      const choice = parsed.choices?.[0];
      if (!choice) continue;

      const text = choice.delta?.content;
      if (text) yield { type: "text", text };

      for (const c of choice.delta?.tool_calls ?? []) {
        const index = c.index ?? 0;
        const slot = pending.get(index) ?? { id: "", name: "", args: "" };
        if (c.id) slot.id = c.id;
        if (c.function?.name) slot.name = c.function.name;
        if (c.function?.arguments) slot.args += c.function.arguments;
        pending.set(index, slot);
      }

      if (choice.finish_reason) stopReason = stopReasonFrom(choice.finish_reason);
    }

    for (const [, slot] of [...pending.entries()].sort((a, b) => a[0] - b[0])) {
      yield { type: "tool_call", call: { id: slot.id, name: slot.name, arguments: slot.args } };
    }

    yield { type: "done", stopReason, usage };
  }
}

/**
 * Yield the `data:` payloads of a server-sent-event stream, minus `[DONE]`.
 *
 * Written here rather than pulled in because it is twenty lines and the two
 * things that make SSE parsing wrong are both about buffering, not about
 * features: an event can straddle a chunk boundary, and a chunk can carry
 * several events. Splitting each chunk independently gets both wrong, and
 * gets them wrong only under load — which is when the bug is hardest to see.
 *
 * Comment lines (`:` — OpenRouter sends `: OPENROUTER PROCESSING` keep-alives)
 * are dropped, per the SSE spec.
 */
async function* sseData(body: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const decoder = new TextDecoder();
  const reader = body.getReader();
  let buffer = "";

  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      let newline: number;
      while ((newline = buffer.indexOf("\n")) !== -1) {
        const line = buffer.slice(0, newline).replace(/\r$/, "");
        buffer = buffer.slice(newline + 1);
        if (!line || line.startsWith(":")) continue;
        if (!line.startsWith("data:")) continue;
        const payload = line.slice("data:".length).trim();
        if (payload === "[DONE]") return;
        yield payload;
      }
    }
  } finally {
    reader.releaseLock();
  }
}
