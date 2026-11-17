import assert from "node:assert/strict";
import { test } from "node:test";
import { parseToml } from "./toml.ts";
import * as T from "./topology.ts";

const SRC = `# system
[[system]]
id    = "site"
nodes = ["controller", "server"]

[[node]]
id         = "controller"
kind       = "mcu"
firmware   = "controller-fw"
interfaces = ["wifi", "i2c0"]

[[node]]
id         = "server"
kind       = "host"
runtime    = "linux-compose"
interfaces = ["eth0"]

[[bus]]
id      = "sensors"
type    = "i2c"
connect = ["controller.i2c0"]

[[bus]]
id      = "lan"
type    = "ip"
broker  = "server"
connect = ["controller.wifi", "server.eth0"]

[[parameter]]
id      = "threshold"
scope   = "controller-fw"
unit    = "ppm"
range   = [400, 2000]
default = 1000
`;
const doc = () => parseToml(SRC);

test("the declaration reads as nodes and buses", () => {
  const t = T.read(doc());
  assert.deepEqual(t.nodes.map((n) => n.id), ["controller", "server"]);
  assert.equal(t.nodes[1].detail, "linux-compose");
  assert.deepEqual(t.buses[1].connect, ["controller.wifi", "server.eth0"]);
  assert.deepEqual(T.endpoints(t), ["controller.wifi", "controller.i2c0", "server.eth0"]);
});

test("layout is a pure function: same declaration, same picture, buses under their nodes", () => {
  const a = T.layout(T.read(doc()));
  const b = T.layout(T.read(doc()));
  assert.deepEqual(a, b);
  assert.equal(a.edges.length, 3);
  const [controller, server] = a.nodes;
  const lan = a.buses.find((x) => x.item.id === "lan")!;
  const mid = (controller.x + controller.w / 2 + server.x + server.w / 2) / 2;
  assert.ok(Math.abs(lan.x + lan.w / 2 - mid) < 120, "centred under what it joins");
  const [s, l] = a.buses;
  assert.ok(Math.abs(s.x - l.x) >= 150, "buses do not overlap");
  assert.ok(a.width > 0 && a.height > 0);
});

test("a bus that names a node that does not exist draws no edge, not a crash", () => {
  const t = T.read(parseToml('[[node]]\nid="a"\nkind="mcu"\n[[bus]]\nid="b"\ntype="i2c"\nconnect=["ghost.x","a.y"]'));
  assert.equal(T.layout(t).edges.length, 1);
});

test("a new MCU names its firmware and joins the first system, as the declaration requires", () => {
  const r = T.addNode(SRC, doc());
  assert.equal(r.id, "node");
  assert.match(r.text, /# system/);
  const d = parseToml(r.text) as { node: { id: string; firmware?: string }[]; system: { nodes: string[] }[] };
  assert.equal(d.node.length, 3);
  assert.equal(d.node[2].firmware, "node");
  assert.deepEqual(d.system[0].nodes, ["controller", "server", "node"]);
  const again = T.addNode(r.text, parseToml(r.text));
  assert.equal(again.id, "node-2");
  const host = T.addNode(SRC, doc(), "host");
  assert.equal((parseToml(host.text) as { node: { firmware?: string }[] }).node[2].firmware, undefined, "a host needs no firmware");
});

test("renaming a node updates every reference, so the declaration stays valid", () => {
  const out = T.renameNode(SRC, doc(), "controller", "sensor-board");
  const d = parseToml(out) as { system: { nodes: string[] }[]; bus: { connect: string[] }[]; parameter: { scope: string }[]; node: { id: string }[] };
  assert.deepEqual(d.node.map((n) => n.id), ["sensor-board", "server"]);
  assert.deepEqual(d.system[0].nodes, ["sensor-board", "server"]);
  assert.deepEqual(d.bus[0].connect, ["sensor-board.i2c0"]);
  assert.deepEqual(d.bus[1].connect, ["sensor-board.wifi", "server.eth0"]);
  assert.equal(d.parameter[0].scope, "controller-fw", "scope names the firmware, which a rename does not change");
  assert.doesNotMatch(out, /"controller"/);
});

test("changing a node's firmware moves the parameters scoped to it", () => {
  const out = T.setFirmware(SRC, doc(), "controller", "ctl");
  const d = parseToml(out) as { node: { firmware?: string }[]; parameter: { scope: string }[] };
  assert.equal(d.node[0].firmware, "ctl");
  assert.equal(d.parameter[0].scope, "ctl");
});

test("renaming the broker moves the broker reference too", () => {
  const out = T.renameNode(SRC, doc(), "server", "pi");
  assert.equal((parseToml(out) as { bus: { broker?: string }[] }).bus[1].broker, "pi");
});

test("removing a node removes what depended on it and nothing else", () => {
  const out = T.removeNode(SRC, doc(), "controller");
  const d = parseToml(out) as { system: { nodes: string[] }[]; bus: { connect: string[] }[]; parameter?: unknown[]; node: { id: string }[] };
  assert.deepEqual(d.node.map((n) => n.id), ["server"]);
  assert.deepEqual(d.system[0].nodes, ["server"]);
  assert.deepEqual(d.bus[0].connect, []);
  assert.deepEqual(d.bus[1].connect, ["server.eth0"]);
  assert.equal(d.parameter, undefined, "the parameter scoped to its firmware goes with it");
  assert.match(out, /# system/);
});

test("removing the broker clears the field rather than leaving a dangling name", () => {
  const out = T.removeNode(SRC, doc(), "server");
  assert.equal((parseToml(out) as { bus: { broker?: string }[] }).bus[1].broker, undefined);
});

test("wiring and unwiring an endpoint", () => {
  const wired = T.connect(SRC, doc(), "sensors", "server.eth0");
  assert.deepEqual((parseToml(wired) as { bus: { connect: string[] }[] }).bus[0].connect, ["controller.i2c0", "server.eth0"]);
  assert.equal(T.connect(wired, parseToml(wired), "sensors", "server.eth0"), wired, "idempotent");
  const cut = T.disconnect(wired, parseToml(wired), "sensors", "controller.i2c0");
  assert.deepEqual((parseToml(cut) as { bus: { connect: string[] }[] }).bus[0].connect, ["server.eth0"]);
});

test("removing a bus", () => {
  const out = T.removeBus(SRC, doc(), "sensors");
  assert.deepEqual(T.read(parseToml(out)).buses.map((b) => b.id), ["lan"]);
});

const PIPE = `name     = "topology"
executor = "fid-topology"
outputs  = [
  "firmware/controller-fw/generated/params.rs",
  "firmware/controller-fw/generated/topics.rs",
  "board/wiring.svg",
]
# note
`;
const outs = (t: string) => (parseToml(t) as { outputs: string[] }).outputs;

test("the language picks the extension of the generated tables", () => {
  assert.deepEqual([undefined, "rust", "C", "c++", "zig"].map(T.languageExt), ["rs", "rs", "h", "h", "json"]);
});

test("a topology that already matches its pipeline leaves it byte-for-byte alone", () => {
  assert.equal(T.syncOutputs(PIPE, T.read(doc())), PIPE);
});

test("adding an MCU adds its two tables; unrelated outputs and comments stay", () => {
  const t = T.read(parseToml(T.addNode(SRC, doc()).text));
  const out = T.syncOutputs(PIPE, t);
  assert.deepEqual(outs(out), [
    "firmware/controller-fw/generated/params.rs",
    "firmware/controller-fw/generated/topics.rs",
    "firmware/node/generated/params.rs",
    "firmware/node/generated/topics.rs",
    "board/wiring.svg",
  ]);
  assert.match(out, /# note/);
});

test("removing a node removes its tables; a language change re-extensions them", () => {
  const gone = T.syncOutputs(PIPE, T.read(parseToml(T.removeNode(SRC, doc(), "controller"))));
  assert.deepEqual(outs(gone), ["board/wiring.svg"]);
  const asC = T.syncOutputs(PIPE, T.read(parseToml(SRC.replace('kind       = "mcu"', 'kind       = "mcu"\nlanguage   = "c"'))));
  assert.deepEqual(outs(asC).slice(0, 2), ["firmware/controller-fw/generated/params.h", "firmware/controller-fw/generated/topics.h"]);
});
