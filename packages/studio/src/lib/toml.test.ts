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
