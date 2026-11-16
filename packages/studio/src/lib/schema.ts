import type { Schema } from "./types.ts";

/**
 * What a form needs to know about one value, read off a JSON Schema.
 *
 * `fid studio schema` emits schemars' output, which uses `$ref` into `$defs`,
 * `anyOf [T, null]` for optional fields and `oneOf` of constants for enums. This
 * flattens those into a small closed set of kinds so the form is a `switch`, not
 * a JSON Schema implementation. Anything it does not recognise becomes `json`:
 * a raw editor, never a missing field.
 */
export type Field =
  | { kind: "object"; description?: string; props: { name: string; field: Field }[] }
  | { kind: "map"; description?: string; value: Field }
  | { kind: "list"; description?: string; item: Field }
  | { kind: "enum"; description?: string; options: string[] }
  | { kind: "string" | "number" | "boolean" | "json"; description?: string };

type S = Record<string, unknown>;

function deref(root: S, node: S, depth = 0): S {
  const ref = node["$ref"];
  if (typeof ref === "string" && ref.startsWith("#/") && depth < 32) {
    let cur: unknown = root;
    for (const part of ref.slice(2).split("/")) cur = (cur as S | undefined)?.[part];
    if (cur && typeof cur === "object") {
      // A `$ref` may carry its own description beside it; keep it.
      const { $ref: _r, ...rest } = node;
      return deref(root, { ...(cur as S), ...rest }, depth + 1);
    }
  }
  return node;
}

/** The one non-null branch of `anyOf`/`oneOf`/`type: [T, "null"]`, if that is what this is. */
function unwrapOptional(root: S, node: S): S {
  const t = node["type"];
  if (Array.isArray(t)) {
    const rest = t.filter((x) => x !== "null");
    if (rest.length === 1) return { ...node, type: rest[0] };
  }
  for (const key of ["anyOf", "oneOf"] as const) {
    const arr = node[key];
    if (!Array.isArray(arr)) continue;
    const live = arr.map((b) => deref(root, b as S)).filter((b) => b["type"] !== "null");
    if (live.length === 1 && arr.length === 2) {
      const { [key]: _k, ...rest } = node;
      return unwrapOptional(root, { ...rest, ...live[0] });
    }
  }
  return node;
}

export function fieldOf(root: Schema, raw: unknown): Field {
  const node = unwrapOptional(root, deref(root, (raw ?? {}) as S));
  const description = typeof node["description"] === "string" ? (node["description"] as string) : undefined;

  const options = (node["enum"] as unknown[] | undefined) ??
    ((node["oneOf"] as S[] | undefined)?.every((b) => "const" in b) ? (node["oneOf"] as S[]).map((b) => b["const"]) : undefined);
  if (options?.every((o) => typeof o === "string")) {
    return { kind: "enum", description, options: options as string[] };
  }

  switch (node["type"]) {
    case "string":
      return { kind: "string", description };
    case "integer":
    case "number":
      return { kind: "number", description };
    case "boolean":
      return { kind: "boolean", description };
    case "array":
      return { kind: "list", description, item: fieldOf(root, node["items"]) };
    case "object": {
      const props = node["properties"] as S | undefined;
      if (props && Object.keys(props).length > 0) {
        return {
          kind: "object",
          description,
          props: Object.entries(props).map(([name, p]) => ({ name, field: fieldOf(root, p) })),
        };
      }
      const add = node["additionalProperties"];
      if (add && typeof add === "object") {
        return { kind: "map", description, value: fieldOf(root, add) };
      }
      return { kind: "json", description };
    }
    default:
      return { kind: "json", description };
  }
}

/** First paragraph of a doc comment, which is what a label's hint should be. */
export function summary(text?: string): string | undefined {
  return text?.split(/\n\s*\n/)[0]?.replace(/\s+/g, " ").trim();
}
