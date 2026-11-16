import { parse } from "smol-toml";

export type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

export function parseToml(text: string): Record<string, Json> {
  return parse(text) as Record<string, Json>;
}

/** Read the value at a path, or `undefined`. */
export function getPath(doc: unknown, path: string[]): Json | undefined {
  let cur: unknown = doc;
  for (const k of path) {
    if (cur === null || typeof cur !== "object" || Array.isArray(cur)) return undefined;
    cur = (cur as Record<string, unknown>)[k];
  }
  return cur as Json | undefined;
}

// ── Writing a value ───────────────────────────────────────────────────────────

const BARE = /^[A-Za-z0-9_-]+$/;

function key(k: string): string {
  return BARE.test(k) ? k : JSON.stringify(k);
}

/** A value as inline TOML. */
export function inline(v: Json): string {
  if (v === null) throw new Error("TOML has no null");
  if (typeof v === "string") return JSON.stringify(v);
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  if (Array.isArray(v)) return `[${v.map(inline).join(", ")}]`;
  const parts = Object.entries(v).map(([k, x]) => `${key(k)} = ${inline(x)}`);
  return parts.length ? `{ ${parts.join(", ")} }` : "{}";
}

// ── Patching text, not re-serialising it ──────────────────────────────────────
//
// A declaration's comments are its documentation and its key order is how people
// read it, so changing one value must not rewrite the other hundred lines.
// Studio therefore patches the text. It supports what a form produces — a
// scalar, a list or an inline table at a `[table]` path — and refuses the rest
// rather than guess: a refused edit is a message, a wrong one is a corrupted
// declaration.

const HEADER = /^\s*\[([^\[\]]+)\]\s*(#.*)?$/;
const ARRAY_HEADER = /^\s*\[\[/;

function segments(header: string): string[] {
  const out: string[] = [];
  let cur = "";
  let q: string | null = null;
  for (const c of header) {
    if (q) {
      if (c === q) q = null;
      else cur += c;
    } else if (c === '"' || c === "'") q = c;
    else if (c === ".") {
      out.push(cur.trim());
      cur = "";
    } else cur += c;
  }
  out.push(cur.trim());
  return out;
}

/** Where a value that starts at `from` in `line` ends: index of the first `#`
 *  outside a string at depth 0, or the line's end. Returns the depth left open
 *  so a multi-line array can continue on the next line. */
function scanValue(line: string, from: number, depth: number): { end: number; depth: number } {
  let q: string | null = null;
  for (let i = from; i < line.length; i++) {
    const c = line[i];
    if (q) {
      if (c === "\\" && q === '"') i++;
      else if (c === q) q = null;
    } else if (c === '"' || c === "'") q = c;
    else if (c === "[" || c === "{") depth++;
    else if (c === "]" || c === "}") depth--;
    else if (c === "#" && depth === 0) return { end: i, depth };
  }
  return { end: line.length, depth };
}

export class EditRefused extends Error {}

/**
 * Set or (with `undefined`) remove the value at `path`, keeping comments.
 * Throws `EditRefused` for layouts it will not touch (arrays of tables, dotted
 * keys), which the caller shows as "edit this one in the raw tab".
 */
export function editToml(text: string, path: string[], value: Json | undefined): string {
  if (path.length === 0) throw new EditRefused("empty path");
  const lines = text.split("\n");
  const table = path.slice(0, -1).join("\u0000");
  const name = path[path.length - 1];
  const rendered = value === undefined ? null : `${key(name)} = ${inline(value)}`;

  // Walk the file; remember the lines of the section we want.
  let current = "";
  let sectionStart = -1; // first line after the header (or 0 for the root)
  let sectionEnd = lines.length; // exclusive
  let found = table === "";
  if (found) sectionStart = 0;
  for (let i = 0; i < lines.length; i++) {
    if (ARRAY_HEADER.test(lines[i])) {
      if (found && sectionStart >= 0 && sectionEnd === lines.length && i > sectionStart) sectionEnd = i;
      current = "\u0001array";
      continue;
    }
    const h = HEADER.exec(lines[i]);
    if (!h) continue;
    if (found && i >= sectionStart && sectionEnd === lines.length && (sectionStart > 0 || table === "")) {
      sectionEnd = i;
    }
    current = segments(h[1]).join("\u0000");
    if (current === table && table !== "") {
      found = true;
      sectionStart = i + 1;
      sectionEnd = lines.length;
    }
  }
  if (table !== "" && current === "\u0001array" && !found) {
    // fall through: the table is simply absent
  }

  if (!found) {
    if (rendered === null) return text; // removing what is not there
    const head = table.split("\u0000").map(key).join(".");
    const sep = text.endsWith("\n") || text === "" ? "" : "\n";
    return `${text}${sep}${text === "" ? "" : "\n"}[${head}]\n${rendered}\n`;
  }

  // Find the key inside [sectionStart, sectionEnd).
  const keyRe = new RegExp(`^(\\s*)(?:${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}|"${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}")\\s*=`);
  for (let i = sectionStart; i < sectionEnd; i++) {
    const m = keyRe.exec(lines[i]);
    if (!m) continue;
    // The value may run over several lines (an array); find where it stops.
    let j = i;
    let { end, depth } = scanValue(lines[i], m[0].length, 0);
    let tail = lines[i].slice(end);
    while (depth > 0 && j + 1 < lines.length) {
      j++;
      ({ end, depth } = scanValue(lines[j], 0, depth));
      tail = lines[j].slice(end);
    }
    if (rendered === null) {
      lines.splice(i, j - i + 1);
    } else {
      const comment = tail.trim().startsWith("#") ? `  ${tail.trim()}` : "";
      lines.splice(i, j - i + 1, `${m[1]}${rendered}${comment}`);
    }
    return lines.join("\n");
  }

  if (rendered === null) return text;
  // Absent key: append after the section's last real line.
  let last = sectionStart - 1;
  for (let i = sectionStart; i < sectionEnd; i++) {
    if (lines[i].trim() !== "" && !lines[i].trim().startsWith("#")) last = i;
  }
  if (table === "" && last < 0) last = -1;
  lines.splice(last + 1, 0, rendered);
  return lines.join("\n");
}
