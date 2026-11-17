import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { discoverBlocks, ensureImport, importSourceFor, insertBlock, moveBlock, pruneImport, readPage, removeBlock, renderAttr, setAttr } from "./blocks.ts";

const PAGE = `"use client";
import { Hero, Cta } from "@/components/ui/sections";
import { Header } from "@/components/header";

// The home page.
export default function Home() {
  const year = 2026;
  return (
    <>
      <Header />
      <Hero eyebrow="Live" title="Build things" tight />
      {/* the pitch */}
      <Cta title={"Start"} count={3} action={<a href="/x">go</a>}>
        <p>Details</p>
      </Cta>
    </>
  );
}
`;

test("a page reads as its list of blocks, with literal props decoded", () => {
  const p = readPage(PAGE);
  assert.deepEqual(p.items.map((i) => [i.kind, i.name]), [["block", "Header"], ["block", "Hero"], ["opaque", "{expression}"], ["block", "Cta"]]);
  const hero = p.items[1];
  assert.deepEqual(hero.attrs.map((a) => [a.name, a.kind, a.value]), [["eyebrow", "string", "Live"], ["title", "string", "Build things"], ["tight", "boolean", true]]);
  const cta = p.items[3];
  assert.deepEqual(cta.attrs.map((a) => [a.kind, a.value]), [["string", "Start"], ["number", 3], ["expression", '<a href="/x">go</a>']]);
  assert.equal(cta.hasChildren, true);
  assert.equal(p.indent, "      ");
  assert.equal(p.imports.Hero, "@/components/ui/sections");
});

test("a file that is not a page says why", () => {
  assert.throws(() => readPage("export const x = 1;"), /no default export/);
  assert.throws(() => readPage("export default function P(){ return null; }"), /does not return JSX/);
  assert.throws(() => readPage("export default function P(){ return <main />; }"), /self-closing/);
});

test("moving a block swaps slices and changes nothing else", () => {
  const out = moveBlock(PAGE, 1, 0);
  const names = readPage(out).items.map((i) => i.name);
  assert.deepEqual(names, ["Hero", "Header", "{expression}", "Cta"]);
  assert.match(out, /\/\/ The home page\./);
  assert.match(out, /const year = 2026;/);
  assert.match(out, /<p>Details<\/p>/, "a block's own content is carried along untouched");
  // Moving there and back is the identity.
  assert.equal(moveBlock(out, 0, 1), PAGE);
});

test("removing a block takes its line and its now-unused import, but not a shared one", () => {
  const out = removeBlock(PAGE, 0);
  assert.deepEqual(readPage(out).items.map((i) => i.name), ["Hero", "{expression}", "Cta"]);
  assert.doesNotMatch(out, /Header/);
  assert.match(out, /import \{ Hero, Cta \}/);
  const noHero = removeBlock(PAGE, 1);
  assert.match(noHero, /import \{ Cta \} from "@\/components\/ui\/sections"/, "the other name in the import stays");
});

test("inserting adds the element and its import, at the start, middle or end", () => {
  const props = { title: "New" };
  for (const [index, expected] of [[0, 0], [2, 2], [99, 4]] as const) {
    const out = insertBlock(PAGE, index, "Feature", "@/components/ui/sections", props);
    const p = readPage(out);
    assert.equal(p.items[expected].name, "Feature", `index ${index}`);
    assert.equal(p.items.length, 5);
    assert.match(out, /Feature[^\n]*title="New"/);
  }
  const out = insertBlock(PAGE, 0, "Feature", "@/components/ui/sections");
  assert.match(out, /import \{ Hero, Cta, Feature \} from "@\/components\/ui\/sections";/, "joins the import from the same module");
  const fresh = insertBlock(PAGE, 0, "Pricing", "@/components/blocks/pricing");
  assert.match(fresh, /import \{ Header \} from "@\/components\/header";\nimport \{ Pricing \} from "@\/components\/blocks\/pricing";/, "a new module gets its own line, after the others");
});

test("inserting into an empty page works", () => {
  const empty = `import { A } from "@/a";\nexport default function P() {\n  return (\n    <>\n    </>\n  );\n}\n`;
  const out = insertBlock(empty, 0, "A", "@/a");
  assert.deepEqual(readPage(out).items.map((i) => i.name), ["A"]);
});

test("editing a prop rewrites that attribute and nothing else", () => {
  const out = setAttr(PAGE, 1, "title", "Changed");
  assert.match(out, /<Hero eyebrow="Live" title="Changed" tight \/>/);
  assert.equal(setAttr(out, 1, "title", "Build things"), PAGE);
});

test("props can be added, switched on, and removed", () => {
  const added = setAttr(PAGE, 0, "variant", "wide");
  assert.match(added, /<Header variant="wide" \/>/);
  const on = setAttr(PAGE, 3, "flag", true);
  assert.match(on, /count=\{3\} action=\{<a href="\/x">go<\/a>\} flag>/);
  const gone = setAttr(PAGE, 1, "tight", undefined);
  assert.match(gone, /<Hero eyebrow="Live" title="Build things" \/>/);
  assert.equal(setAttr(PAGE, 1, "nope", undefined), PAGE);
});

test("attributes are rendered so they survive a round trip, whatever the text", () => {
  for (const text of ['plain', 'has "quotes"', "curly {braces}", "line\nbreak", "a & b", "<tag>", "back\\slash"]) {
    const out = setAttr(PAGE, 0, "label", text);
    const a = readPage(out).items[0].attrs.find((x) => x.name === "label")!;
    assert.equal(a.value, text, JSON.stringify(text));
  }
  assert.equal(renderAttr("n", 2), "n={2}");
  assert.equal(renderAttr("b", false), "b={false}");
});

test("an import is added once and removed only when nothing uses the name", () => {
  const src = `import { A } from "@/a";\nexport default function P(){ return <><A /></>; }\n`;
  assert.equal(ensureImport(src, "A", "@/elsewhere"), src);
  assert.equal(pruneImport(src, "A"), src, "still used, so it stays");
  const unused = `import { A, B } from "@/a";\nexport default function P(){ return <><B /></>; }\n`;
  assert.match(pruneImport(unused, "A"), /import \{ B \} from "@\/a"/);
});

test("'use client' stays first when the first import is added", () => {
  const out = ensureImport(`"use client";\nexport default function P(){ return <></>; }\n`, "A", "@/a");
  assert.match(out, /^"use client";\n\nimport \{ A \} from "@\/a";/);
});

test("components and their props are discovered from the source", () => {
  const src = `
    import type * as React from "react";
    export interface HeroProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
      /** The small line above. */
      eyebrow?: string;
      title: React.ReactNode;
      size: "sm" | "lg";
      count?: number;
      wide?: boolean;
      action: () => void;
      className?: string;
    }
    export function Hero({ eyebrow, title }: HeroProps) { return null; }
    export const Note = ({ text }: { text: string }) => null;
    export function helper() {}
    function Hidden() {}
  `;
  const defs = discoverBlocks("apps/web/src/components/ui/sections.tsx", src);
  assert.deepEqual(defs.map((d) => d.name), ["Hero", "Note"]);
  assert.equal(defs[0].importSource, "@/components/ui/sections");
  const by = Object.fromEntries(defs[0].props.map((p) => [p.name, p]));
  assert.equal(by.eyebrow.type, "string");
  assert.equal(by.eyebrow.required, false);
  assert.equal(by.eyebrow.doc, "The small line above.");
  assert.equal(by.title.type, "string", "ReactNode is edited as text");
  assert.deepEqual(by.size, { name: "size", type: "enum", options: ["sm", "lg"], required: true, doc: undefined });
  assert.equal(by.count.type, "number");
  assert.equal(by.wide.type, "boolean");
  assert.equal(by.action.type, "other");
  assert.equal(by.className, undefined, "plumbing is not offered");
  assert.deepEqual(defs[1].props.map((p) => p.name), ["text"]);
});

test("the platform's own sections are found with their real props", () => {
  const real = readFileSync(new URL("../../../../crates/fiducial-cli/components/react/sections.tsx", import.meta.url), "utf8");
  const defs = discoverBlocks("apps/web/src/components/ui/sections.tsx", real);
  const names = defs.map((d) => d.name);
  for (const n of ["Hero", "Cta", "Feature", "StatusCard"]) assert.ok(names.includes(n), n);
  const hero = defs.find((d) => d.name === "Hero")!;
  assert.ok(hero.props.length >= 2, "Hero has props");
  assert.ok(hero.props.every((p) => p.name !== "className"));
});

test("import paths come from the file location", () => {
  assert.equal(importSourceFor("apps/web/src/components/blocks/pricing.tsx"), "@/components/blocks/pricing");
  assert.equal(importSourceFor("components/x.tsx"), "components/x");
});
