import assert from "node:assert/strict";
import { test } from "node:test";
import { fieldOf, summary } from "./schema.ts";

const root = {
  $defs: {
    Mode: { oneOf: [{ const: "a" }, { const: "b" }] },
    Inner: { type: "object", properties: { n: { type: "integer" } } },
  },
};

test("$ref and optionals resolve to the plain kind", () => {
  const f = fieldOf(root, { anyOf: [{ $ref: "#/$defs/Inner" }, { type: "null" }] });
  assert.equal(f.kind, "object");
  const g = fieldOf(root, { type: ["string", "null"] });
  assert.equal(g.kind, "string");
});

test("a oneOf of constants is an enum", () => {
  assert.deepEqual(fieldOf(root, { $ref: "#/$defs/Mode" }), { kind: "enum", description: undefined, options: ["a", "b"] });
});

test("maps, lists and the unknown", () => {
  assert.equal(fieldOf(root, { type: "object", additionalProperties: { type: "string" } }).kind, "map");
  assert.equal(fieldOf(root, { type: "array", items: { type: "string" } }).kind, "list");
  assert.equal(fieldOf(root, { weird: true }).kind, "json");
});

test("a description beside a $ref survives", () => {
  const f = fieldOf(root, { $ref: "#/$defs/Inner", description: "mine" });
  assert.equal(f.description, "mine");
});

test("summary keeps the first paragraph", () => {
  assert.equal(summary("One line\nwrapped.\n\nSecond."), "One line wrapped.");
});
