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

/** The document with the value at `path` replaced (or created). Pure. */
export function setPath(doc: Record<string, Json>, path: string[], value: Json): Record<string, Json> {
  const [head, ...rest] = path;
  if (head === undefined) return doc;
  if (rest.length === 0) return { ...doc, [head]: value };
  const child = doc[head];
  const next = child !== null && typeof child === "object" && !Array.isArray(child) ? child : {};
  return { ...doc, [head]: setPath(next as Record<string, Json>, rest, value) };
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

  const out = setKeyInRange(lines, sectionStart, sectionEnd, name, value);
  return out.join("\n");
}

const escapeRe = (x: string) => x.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Set or remove `name` among `lines[start, end)`: replace its value in place
 * (keeping a trailing comment), or add it after the range's last real line.
 */
function setKeyInRange(lines: string[], start: number, end: number, name: string, value: Json | undefined): string[] {
  const rendered = value === undefined ? null : `${key(name)} = ${inline(value)}`;
  const keyRe = new RegExp(`^(\\s*)(?:${escapeRe(name)}|"${escapeRe(name)}")\\s*=`);
  const out = [...lines];
  for (let i = start; i < end; i++) {
    const m = keyRe.exec(out[i]);
    if (!m) continue;
    // The value may run over several lines (an array); find where it stops.
    let j = i;
    let { end: stop, depth } = scanValue(out[i], m[0].length, 0);
    let tail = out[i].slice(stop);
    while (depth > 0 && j + 1 < out.length) {
      j++;
      ({ end: stop, depth } = scanValue(out[j], 0, depth));
      tail = out[j].slice(stop);
    }
    if (rendered === null) out.splice(i, j - i + 1);
    else {
      const comment = tail.trim().startsWith("#") ? `  ${tail.trim()}` : "";
      // A list that was laid out over several lines stays that way, so editing
      // one entry does not collapse the author's formatting into one long line.
      if (j > i && Array.isArray(value) && value.length > 0) {
        const lead = m[1];
        const body = value.map((v) => `${lead}  ${inline(v)},`);
        const head = `${lead}${key(name)} = [`;
        out.splice(i, j - i + 1, head, ...body, `${lead}]${comment}`);
      } else {
        out.splice(i, j - i + 1, `${m[1]}${rendered}${comment}`);
      }
    }
    return out;
  }
  if (rendered === null) return out;
  // Absent key: append after the range's last real line.
  let last = start - 1;
  for (let i = start; i < end; i++) {
    if (out[i].trim() !== "" && !out[i].trim().startsWith("#")) last = i;
  }
  out.splice(last + 1, 0, rendered);
  return out;
}

// ── Arrays of tables: `[[node]]`, `[[bus]]` ───────────────────────────────────
//
// A topology is a list of blocks, not one table, and a person edits one block at
// a time: add a node, change a bus, remove a parameter. These work on the n-th
// `[[name]]` block and leave every other line, comments included, alone.

interface Block {
  /** Index of the `[[name]]` line. */
  header: number;
  /** First line after the header. */
  start: number;
  /** One past the block's last line (the next header, or the end). */
  end: number;
}

function blocksOf(lines: string[], name: string): Block[] {
  const open = new RegExp(`^\\s*\\[\\[\\s*${escapeRe(name)}\\s*\\]\\]`);
  const any = /^\s*\[/;
  const found: Block[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (!open.test(lines[i])) continue;
    let end = i + 1;
    while (end < lines.length && !any.test(lines[end])) end++;
    found.push({ header: i, start: i + 1, end });
  }
  return found;
}

/** How many `[[name]]` blocks there are. */
export function blockCount(text: string, name: string): number {
  return blocksOf(text.split("\n"), name).length;
}

/** Set (or with `undefined`, remove) a key in the `index`-th `[[name]]` block. */
export function editBlock(text: string, name: string, index: number, field: string, value: Json | undefined): string {
  const lines = text.split("\n");
  const b = blocksOf(lines, name)[index];
  if (!b) throw new EditRefused(`there is no [[${name}]] number ${index + 1}`);
  // Keep trailing blank lines (the gap before the next block) out of the range,
  // so a new key lands beside the others rather than after the gap.
  let end = b.end;
  while (end > b.start && lines[end - 1].trim() === "") end--;
  return setKeyInRange(lines, b.start, end, field, value).join("\n");
}

/** Append a `[[name]]` block with these keys. */
export function addBlock(text: string, name: string, entries: Record<string, Json>): string {
  const body = Object.entries(entries).map(([k, v]) => `${key(k)} = ${inline(v)}`);
  const trimmed = text.replace(/\n*$/, "");
  return `${trimmed}${trimmed === "" ? "" : "\n\n"}[[${name}]]\n${body.join("\n")}\n`;
}

/**
 * Remove the `index`-th `[[name]]` block. Comment lines directly above it (no
 * blank line between) describe it and go with it.
 */
export function removeBlock(text: string, name: string, index: number): string {
  const lines = text.split("\n");
  const b = blocksOf(lines, name)[index];
  if (!b) throw new EditRefused(`there is no [[${name}]] number ${index + 1}`);
  let from = b.header;
  while (from > 0 && lines[from - 1].trim().startsWith("#")) from--;
  let to = b.end;
  while (to < lines.length && lines[to].trim() === "") to++;
  // Leave one blank line between what remains above and below.
  const hadGapAbove = from > 0 && lines[from - 1].trim() === "";
  lines.splice(from, to - from);
  if (hadGapAbove && from < lines.length && lines[from]?.trim() !== "") lines.splice(from, 0, "");
  return lines.join("\n").replace(/\n*$/, "\n");
}
