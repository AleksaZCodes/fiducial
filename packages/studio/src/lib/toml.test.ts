import assert from "node:assert/strict";
import { test } from "node:test";
import { editToml, getPath, inline, parseToml } from "./toml.ts";

const doc = `# the product
name_top = "x"   # root key

[product]
name = "demo"   # shown in the header
tags = [
  "a",   # first
  "b",
]

[design]
github = "a/b"

[design.registries]
"@x" = "https://e.com/{name}.json"
`;

const val = (t: string, ...p: string[]) => getPath(parseToml(t), p);

test("editing one value keeps every comment, even on that line", () => {
  const out = editToml(doc, ["product", "name"], "renamed");
  assert.equal(val(out, "product", "name"), "renamed");
  assert.match(out, /name = "renamed"  # shown in the header/);
  assert.match(out, /# the product/);
});

test("a multi-line array is replaced as one value", () => {
  const out = editToml(doc, ["product", "tags"], ["z"]);
  assert.deepEqual(val(out, "product", "tags"), ["z"]);
  assert.equal(val(out, "design", "github"), "a/b");
});

test("a root key and a nested table are told apart", () => {
  const out = editToml(doc, ["name_top"], "y");
  assert.equal(val(out, "name_top"), "y");
  assert.equal(val(out, "product", "name"), "demo");
});

test("a new key lands in its own section, not at the end", () => {
  const out = editToml(doc, ["design", "extra"], true);
  assert.equal(val(out, "design", "extra"), true);
  assert.ok(out.indexOf("extra") < out.indexOf("[design.registries]"));
  assert.equal(val(out, "design", "registries", "@x"), "https://e.com/{name}.json");
});

test("a table that is absent is created", () => {
  const out = editToml(doc, ["brand", "domain"], "example.com");
  assert.equal(val(out, "brand", "domain"), "example.com");
  assert.equal(val(out, "product", "name"), "demo");
});

test("a quoted key is found and edited", () => {
  const out = editToml(doc, ["design", "registries", "@x"], "https://n.com/{name}.json");
  assert.equal(val(out, "design", "registries", "@x"), "https://n.com/{name}.json");
});

test("removing a key removes its line only", () => {
  const out = editToml(doc, ["design", "github"], undefined);
  assert.equal(val(out, "design", "github"), undefined);
  assert.equal(val(out, "product", "name"), "demo");
});

test("values render as TOML", () => {
  assert.equal(inline({ a: [1, "x"], b: true }), '{ a = [1, "x"], b = true }');
  assert.equal(inline('say "hi"'), '"say \\"hi\\""');
});

const topo = `# the system
[[system]]
id = "site"

# the controller
[[node]]
id = "a"
kind = "mcu"   # small

[[node]]
id = "b"
kind = "host"

[[bus]]
id = "x"
`;

import { addBlock, blockCount, editBlock, removeBlock } from "./toml.ts";

test("blocks are counted and one is edited without touching the rest", () => {
  assert.equal(blockCount(topo, "node"), 2);
  const out = editBlock(topo, "node", 1, "kind", "mcu");
  const doc = parseToml(out) as { node: { id: string; kind: string }[] };
  assert.deepEqual(doc.node.map((n) => n.kind), ["mcu", "mcu"]);
  assert.match(out, /kind = "mcu"   # small/);
  assert.match(out, /# the controller/);
});

test("a new key lands inside its block, before the gap", () => {
  const out = editBlock(topo, "node", 0, "target", "rp2040");
  const doc = parseToml(out) as { node: { id: string; target?: string }[] };
  assert.equal(doc.node[0].target, "rp2040");
  assert.equal(doc.node[1].target, undefined);
});

test("a key can be removed from one block", () => {
  const out = editBlock(topo, "node", 0, "kind", undefined);
  assert.equal((parseToml(out) as { node: { kind?: string }[] }).node[0].kind, undefined);
});

test("a block is appended with its keys", () => {
  const out = addBlock(topo, "node", { id: "c", kind: "mcu", interfaces: ["i2c0"] });
  const doc = parseToml(out) as { node: { id: string; interfaces?: string[] }[] };
  assert.equal(doc.node.length, 3);
  assert.deepEqual(doc.node[2].interfaces, ["i2c0"]);
  assert.match(out, /# the controller/);
});

test("removing a block takes its own comment and leaves the others", () => {
  const out = removeBlock(topo, "node", 0);
  const doc = parseToml(out) as { node: { id: string }[]; system: unknown[]; bus: unknown[] };
  assert.deepEqual(doc.node.map((n) => n.id), ["b"]);
  assert.doesNotMatch(out, /# the controller/);
  assert.match(out, /# the system/);
  assert.equal(doc.bus.length, 1);
});

test("a block that is not there is refused, not guessed at", () => {
  assert.throws(() => editBlock(topo, "node", 5, "id", "z"), /no \[\[node\]\] number 6/);
  assert.throws(() => removeBlock(topo, "parameter", 0), /no \[\[parameter\]\]/);
});

test("adding to an empty file works", () => {
  assert.deepEqual(JSON.parse(JSON.stringify(parseToml(addBlock("", "node", { id: "a" })))), { node: [{ id: "a" }] });
});

test("a list laid out over several lines stays that way when an entry changes", () => {
  const out = editToml('outputs = [\n  "a",\n  "b",\n]  # files\n', ["outputs"], ["a", "b", "c"]);
  assert.equal(out, 'outputs = [\n  "a",\n  "b",\n  "c",\n]  # files\n');
});
