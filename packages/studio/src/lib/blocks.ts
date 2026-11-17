import { parse } from "@babel/parser";

/**
 * Pages as code, edited as code.
 *
 * A page is an ordinary TSX file whose default export returns a list of block
 * components: `<Hero … /> <Features … /> <Cta … />`. Studio reads that list with
 * a real parser and changes it by **patching the source text**: moving a block
 * swaps two slices, removing one deletes its lines, editing a prop rewrites that
 * one attribute. Every byte outside the change is untouched, so the file stays
 * the product's own code — reviewable in a diff, editable by hand, with no
 * Studio-only format to lose.
 *
 * Nothing here stores a layout. There is no JSON tree the page is rendered from
 * at run time; deleting Studio leaves a normal React page.
 */

// Babel's nodes are plain objects; only the fields used here are named.
interface N {
  type: string;
  start: number;
  end: number;
  [k: string]: any; // eslint-disable-line @typescript-eslint/no-explicit-any
}

function ast(src: string): N {
  return parse(src, { sourceType: "module", plugins: ["jsx", "typescript"] }).program as unknown as N;
}

function walk(node: unknown, visit: (n: N) => void): void {
  if (!node || typeof node !== "object") return;
  if (Array.isArray(node)) return node.forEach((c) => walk(c, visit));
  const n = node as N;
  if (typeof n.type === "string") visit(n);
  for (const [k, v] of Object.entries(n)) {
    if (k === "loc" || k === "leadingComments" || k === "trailingComments" || k === "innerComments") continue;
    walk(v, visit);
  }
}

export class PageError extends Error {}

// ── Reading a page ────────────────────────────────────────────────────────────

export interface Attr {
  name: string;
  kind: "string" | "number" | "boolean" | "expression";
  value: string | number | boolean;
  start: number;
  end: number;
}

export interface Item {
  /** `block`: a component Studio can edit. `opaque`: anything else, moved but never changed. */
  kind: "block" | "opaque";
  /** Component name, or a short description for opaque items. */
  name: string;
  start: number;
  end: number;
  attrs: Attr[];
  /** The element has content between its tags, which Studio leaves as written. */
  hasChildren: boolean;
  /** Where the opening tag's name ends: attributes are added after the last one. */
  nameEnd: number;
}

export interface Page {
  items: Item[];
  /** Local name → where it was imported from. */
  imports: Record<string, string>;
  /** Whitespace in front of each item, so a new one lines up. */
  indent: string;
  /** Where the container's opening tag ends. */
  openEnd: number;
  closeStart: number;
}

function tagName(n: N): string {
  const t = n.openingElement.name as N;
  if (t.type === "JSXIdentifier") return t.name;
  if (t.type === "JSXMemberExpression") return `${tagNameOf(t.object)}.${t.property.name}`;
  return "?";
}
function tagNameOf(n: N): string {
  return n.type === "JSXIdentifier" ? n.name : `${tagNameOf(n.object)}.${n.property.name}`;
}

function attrOf(src: string, a: N): Attr | null {
  if (a.type !== "JSXAttribute" || a.name.type !== "JSXIdentifier") return null;
  const name = a.name.name as string;
  const base = { name, start: a.start, end: a.end };
  const v = a.value as N | null;
  if (v === null) return { ...base, kind: "boolean", value: true };
  if (v.type === "StringLiteral") return { ...base, kind: "string", value: v.value as string };
  if (v.type === "JSXExpressionContainer") {
    const e = v.expression as N;
    if (e.type === "StringLiteral") return { ...base, kind: "string", value: e.value };
    if (e.type === "TemplateLiteral" && e.expressions.length === 0) return { ...base, kind: "string", value: e.quasis[0].value.cooked as string };
    if (e.type === "NumericLiteral") return { ...base, kind: "number", value: e.value };
    if (e.type === "BooleanLiteral") return { ...base, kind: "boolean", value: e.value };
    return { ...base, kind: "expression", value: src.slice(e.start, e.end) };
  }
  return { ...base, kind: "expression", value: src.slice(v.start, v.end) };
}

/** The JSX a page's default export returns. */
function pageRoot(program: N): N {
  const decl = program.body.find((s: N) => s.type === "ExportDefaultDeclaration") as N | undefined;
  if (!decl) throw new PageError("This file has no default export, so it is not a page.");
  let fn = decl.declaration as N;
  if (fn.type === "Identifier") {
    const target = program.body.find(
      (s: N) => (s.type === "FunctionDeclaration" && s.id?.name === fn.name) ||
        (s.type === "VariableDeclaration" && s.declarations.some((d: N) => d.id.name === fn.name)),
    ) as N | undefined;
    if (!target) throw new PageError(`Could not find \`${fn.name}\`, the default export.`);
    fn = target.type === "VariableDeclaration" ? (target.declarations.find((d: N) => d.id.name === fn.name).init as N) : target;
  }
  let body: N | undefined;
  if (fn.type === "ArrowFunctionExpression" && fn.body.type !== "BlockStatement") body = fn.body;
  else {
    const block = fn.body as N | undefined;
    const ret = block?.body?.find((s: N) => s.type === "ReturnStatement") as N | undefined;
    body = ret?.argument as N | undefined;
  }
  if (!body || (body.type !== "JSXFragment" && body.type !== "JSXElement")) {
    throw new PageError("The page does not return JSX directly. Studio edits a page that returns `<>…</>` or `<main>…</main>` of blocks.");
  }
  return body;
}

export function readPage(src: string): Page {
  const program = ast(src);
  const root = pageRoot(program);
  const open = (root.type === "JSXFragment" ? root.openingFragment : root.openingElement) as N;
  const close = (root.type === "JSXFragment" ? root.closingFragment : root.closingElement) as N | null;
  if (!close) throw new PageError("The page's outer element is self-closing, so there is nowhere to put blocks.");

  const items: Item[] = [];
  for (const ch of root.children as N[]) {
    if (ch.type === "JSXText") {
      if (ch.value.trim() !== "") items.push({ kind: "opaque", name: "text", start: ch.start, end: ch.end, attrs: [], hasChildren: false, nameEnd: ch.start });
    } else if (ch.type === "JSXElement") {
      const name = tagName(ch);
      const lower = /^[a-z]/.test(name);
      items.push({
        kind: lower ? "opaque" : "block",
        name: lower ? `<${name}>` : name,
        start: ch.start,
        end: ch.end,
        attrs: lower ? [] : (ch.openingElement.attributes as N[]).map((a) => attrOf(src, a)).filter((a): a is Attr => a !== null),
        hasChildren: !ch.openingElement.selfClosing,
        nameEnd: ch.openingElement.name.end,
      });
    } else {
      items.push({ kind: "opaque", name: ch.type === "JSXExpressionContainer" ? "{expression}" : "fragment", start: ch.start, end: ch.end, attrs: [], hasChildren: false, nameEnd: ch.start });
    }
  }

  const imports: Record<string, string> = {};
  for (const s of program.body as N[]) {
    if (s.type !== "ImportDeclaration") continue;
    for (const sp of s.specifiers as N[]) imports[sp.local.name] = s.source.value;
  }

  let indent = "      ";
  if (items[0]) {
    const lineStart = src.lastIndexOf("\n", items[0].start - 1) + 1;
    const lead = src.slice(lineStart, items[0].start);
    if (/^[ \t]*$/.test(lead)) indent = lead;
  }
  return { items, imports, indent, openEnd: open.end, closeStart: close.start };
}

// ── Editing a page: each returns new source text ──────────────────────────────

/** An attribute as JSX source. */
export function renderAttr(name: string, value: string | number | boolean): string {
  if (typeof value === "boolean") return value ? name : `${name}={false}`;
  if (typeof value === "number") return `${name}={${value}}`;
  return /^[^"{}\\<>&\n\r]*$/.test(value) ? `${name}="${value}"` : `${name}={${JSON.stringify(value)}}`;
}

function element(name: string, props: Record<string, string | number | boolean>): string {
  const attrs = Object.entries(props).map(([k, v]) => renderAttr(k, v));
  return `<${[name, ...attrs].join(" ")} />`;
}

/** Add `import { name } from "source"` unless `name` is already imported. */
export function ensureImport(src: string, name: string, source: string): string {
  const program = ast(src);
  const imports = (program.body as N[]).filter((s) => s.type === "ImportDeclaration");
  if (imports.some((s) => (s.specifiers as N[]).some((sp) => sp.local.name === name))) return src;
  // Join an existing value import from the same module rather than adding a
  // second line for it, as a person would.
  const same = imports.find((s) => s.source.value === source && s.importKind !== "type" && (s.specifiers as N[]).some((sp) => sp.type === "ImportSpecifier"));
  if (same) {
    const lastSpec = [...(same.specifiers as N[])].reverse().find((sp) => sp.type === "ImportSpecifier")!;
    return `${src.slice(0, lastSpec.end)}, ${name}${src.slice(lastSpec.end)}`;
  }
  const line = `import { ${name} } from "${source}";`;
  if (imports.length === 0) {
    // After a leading directive such as "use client".
    const dir = (program.directives as N[] | undefined)?.at(-1);
    return dir ? `${src.slice(0, dir.end)}\n\n${line}${src.slice(dir.end)}` : `${line}\n${src}`;
  }
  const last = imports[imports.length - 1];
  return `${src.slice(0, last.end)}\n${line}${src.slice(last.end)}`;
}

/** Remove the import of `name` when nothing in the file uses it any more. */
export function pruneImport(src: string, name: string): string {
  const program = ast(src);
  let used = false;
  walk(program, (n) => {
    if ((n.type === "JSXIdentifier" || n.type === "Identifier") && n.name === name) {
      // The import's own specifier is an Identifier too; skip declarations.
      used = used || !isImportSpecifierId(program, n);
    }
  });
  if (used) return src;
  for (const s of program.body as N[]) {
    if (s.type !== "ImportDeclaration") continue;
    const specs = s.specifiers as N[];
    const i = specs.findIndex((sp) => sp.local.name === name);
    if (i === -1) continue;
    if (specs.length === 1) {
      const from = src.lastIndexOf("\n", s.start - 1) + 1;
      const to = src[s.end] === "\n" ? s.end + 1 : s.end;
      return src.slice(0, s.start === from ? from : s.start) + src.slice(to);
    }
    const sp = specs[i];
    const next = specs[i + 1];
    return next ? src.slice(0, sp.start) + src.slice(next.start) : src.slice(0, specs[i - 1].end) + src.slice(sp.end);
  }
  return src;
}

function isImportSpecifierId(program: N, id: N): boolean {
  for (const s of program.body as N[]) {
    if (s.type !== "ImportDeclaration") continue;
    for (const sp of s.specifiers as N[]) {
      if (id.start >= sp.start && id.end <= sp.end) return true;
    }
  }
  return false;
}

/** Insert a new block at `index` (0 = first), importing it if need be. */
export function insertBlock(
  src: string,
  index: number,
  name: string,
  importSource: string,
  props: Record<string, string | number | boolean> = {},
): string {
  const page = readPage(src);
  const text = element(name, props);
  let out: string;
  if (page.items.length === 0) out = `${src.slice(0, page.openEnd)}\n${page.indent}${text}\n${src.slice(page.openEnd).replace(/^[ \t]*\n?/, "")}`;
  else if (index >= page.items.length) {
    const last = page.items[page.items.length - 1];
    out = `${src.slice(0, last.end)}\n${page.indent}${text}${src.slice(last.end)}`;
  } else {
    const at = page.items[index].start;
    out = `${src.slice(0, at)}${text}\n${page.indent}${src.slice(at)}`;
  }
  return ensureImport(out, name, importSource);
}

/** Delete the block at `index`, with its line, and its import if it was the last use. */
export function removeBlock(src: string, index: number): string {
  const page = readPage(src);
  const it = page.items[index];
  if (!it) throw new PageError(`There is no item ${index + 1} on this page.`);
  let from = it.start;
  while (from > 0 && (src[from - 1] === " " || src[from - 1] === "\t")) from--;
  if (from > 0 && src[from - 1] === "\n") from--;
  const out = src.slice(0, from) + src.slice(it.end);
  return it.kind === "block" ? pruneImport(out, it.name) : out;
}

/** Move the item at `from` to position `to`: only the item slices swap places. */
export function moveBlock(src: string, from: number, to: number): string {
  const page = readPage(src);
  const n = page.items.length;
  if (from < 0 || from >= n || to < 0 || to >= n) throw new PageError("That move is outside the page.");
  if (from === to) return src;
  const slices = page.items.map((i) => src.slice(i.start, i.end));
  const [moved] = slices.splice(from, 1);
  slices.splice(to, 0, moved);
  let out = src;
  for (let i = n - 1; i >= 0; i--) {
    const it = page.items[i];
    out = out.slice(0, it.start) + slices[i] + out.slice(it.end);
  }
  return out;
}

/** Set (or with `undefined`, remove) one attribute of the block at `index`. */
export function setAttr(src: string, index: number, name: string, value: string | number | boolean | undefined): string {
  const it = readPage(src).items[index];
  if (!it || it.kind !== "block") throw new PageError("Only a component can have its props edited.");
  const existing = it.attrs.find((a) => a.name === name);
  if (existing) {
    if (value === undefined) {
      let from = existing.start;
      while (from > 0 && /\s/.test(src[from - 1])) from--;
      return src.slice(0, from) + src.slice(existing.end);
    }
    return src.slice(0, existing.start) + renderAttr(name, value) + src.slice(existing.end);
  }
  if (value === undefined) return src;
  const at = it.attrs.length ? it.attrs[it.attrs.length - 1].end : it.nameEnd;
  return `${src.slice(0, at)} ${renderAttr(name, value)}${src.slice(at)}`;
}

// ── What blocks exist ─────────────────────────────────────────────────────────

export interface BlockProp {
  name: string;
  type: "string" | "number" | "boolean" | "enum" | "other";
  options?: string[];
  required: boolean;
  doc?: string;
}

export interface BlockDef {
  name: string;
  /** What a page imports it from, e.g. `@/components/ui/sections`. */
  importSource: string;
  props: BlockProp[];
}

/** Props that are plumbing, not content: they are not offered as fields. */
const PLUMBING = new Set(["className", "children", "key", "ref", "style"]);

/** `apps/web/src/components/ui/sections.tsx` → `@/components/ui/sections`. */
export function importSourceFor(file: string): string {
  const noExt = file.replace(/\.(tsx|ts|jsx|js)$/, "");
  const i = noExt.lastIndexOf("src/");
  return i === -1 ? noExt : `@/${noExt.slice(i + 4)}`;
}

function typeOf(t: N | undefined): Pick<BlockProp, "type" | "options"> {
  if (!t) return { type: "other" };
  const a = t.type === "TSTypeAnnotation" ? (t.typeAnnotation as N) : t;
  switch (a.type) {
    case "TSStringKeyword": return { type: "string" };
    case "TSNumberKeyword": return { type: "number" };
    case "TSBooleanKeyword": return { type: "boolean" };
    case "TSTypeReference": {
      const n = a.typeName as N;
      const name = n.type === "Identifier" ? n.name : `${n.left.name}.${n.right.name}`;
      // Content that is text in practice is edited as text.
      return /(^|\.)ReactNode$/.test(name) ? { type: "string" } : { type: "other" };
    }
    case "TSUnionType": {
      const lits = (a.types as N[]).filter((x) => x.type === "TSLiteralType" && x.literal.type === "StringLiteral");
      if (lits.length === a.types.length) return { type: "enum", options: lits.map((x) => x.literal.value as string) };
      // `string | undefined` and friends.
      const rest = (a.types as N[]).filter((x) => x.type !== "TSUndefinedKeyword" && x.type !== "TSNullKeyword");
      return rest.length === 1 ? typeOf(rest[0]) : { type: "other" };
    }
    default: return { type: "other" };
  }
}

function membersOf(program: N, t: N): N[] {
  if (t.type === "TSTypeLiteral") return t.members as N[];
  if (t.type === "TSTypeReference" && t.typeName.type === "Identifier") {
    const name = t.typeName.name as string;
    for (const s of program.body as N[]) {
      const d = s.type === "ExportNamedDeclaration" ? (s.declaration as N | null) : s;
      if (!d) continue;
      if (d.type === "TSInterfaceDeclaration" && d.id.name === name) return d.body.body as N[];
      if (d.type === "TSTypeAliasDeclaration" && d.id.name === name) return membersOf(program, d.typeAnnotation as N);
    }
  }
  if (t.type === "TSIntersectionType") return (t.types as N[]).flatMap((x) => membersOf(program, x));
  return [];
}

/** The components a file exports, with the props each one takes. */
export function discoverBlocks(file: string, src: string): BlockDef[] {
  let program: N;
  try {
    program = ast(src);
  } catch {
    return [];
  }
  const importSource = importSourceFor(file);
  const out: BlockDef[] = [];
  const consider = (name: string, fn: N) => {
    if (!/^[A-Z]/.test(name)) return;
    const param = (fn.params as N[])[0];
    const ann = param?.typeAnnotation?.typeAnnotation as N | undefined;
    const props: BlockProp[] = [];
    if (ann) {
      for (const m of membersOf(program, ann)) {
        if (m.type !== "TSPropertySignature" || m.key.type !== "Identifier" || PLUMBING.has(m.key.name)) continue;
        const doc = (m.leadingComments as N[] | undefined)?.at(-1)?.value as string | undefined;
        props.push({
          name: m.key.name,
          ...typeOf(m.typeAnnotation),
          required: !m.optional,
          doc: doc?.replace(/^\*+/, "").split("\n").map((l) => l.replace(/^\s*\*?\s?/, "")).join(" ").trim() || undefined,
        });
      }
    }
    out.push({ name, importSource, props });
  };
  for (const s of program.body as N[]) {
    if (s.type !== "ExportNamedDeclaration" || !s.declaration) continue;
    const d = s.declaration as N;
    if (d.type === "FunctionDeclaration" && d.id) consider(d.id.name, d);
    if (d.type === "VariableDeclaration") {
      for (const v of d.declarations as N[]) {
        const init = v.init as N | undefined;
        if (v.id.type === "Identifier" && init && (init.type === "ArrowFunctionExpression" || init.type === "FunctionExpression")) consider(v.id.name, init);
      }
    }
  }
  return out;
}
