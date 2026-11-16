import { addBlock, blockCount, editBlock, editToml, getPath, parseToml, removeBlock, type Json } from "./toml.ts";

/**
 * The topology as a diagram, and every edit to it as a change to `topology.toml`.
 *
 * The diagram is **not a second source**. Nothing here is stored: `read` turns
 * the declaration into nodes and buses, `layout` places them (positions are
 * derived, never saved), and each operation returns new TOML text. Deleting the
 * app loses nothing, and `fid derive` reads the same file the diagram edits.
 */
export interface TNode {
  id: string;
  kind: string;
  language?: string;
  /** The firmware member a parameter's `scope` names; not the node's id. */
  firmware?: string;
  interfaces: string[];
  /** One-line detail for the box: runtime, or the apps it runs. */
  detail: string;
}

export interface TBus {
  id: string;
  type: string;
  protocol?: string;
  /** `node.interface` endpoints. */
  connect: string[];
  broker?: string;
}

export interface Topo {
  nodes: TNode[];
  buses: TBus[];
}

type Doc = Record<string, Json>;

const strs = (v: Json | undefined): string[] => (Array.isArray(v) ? v.map(String) : []);
const str = (v: Json | undefined): string | undefined => (typeof v === "string" ? v : undefined);
const tables = (v: Json | undefined): Doc[] => (Array.isArray(v) ? (v as Doc[]) : []);

export function read(doc: Doc): Topo {
  return {
    nodes: tables(doc.node).map((n) => ({
      id: str(n.id) ?? "?",
      kind: str(n.kind) ?? "?",
      firmware: str(n.firmware),
      language: str(n.language),
      interfaces: strs(n.interfaces),
      detail: str(n.runtime) ?? strs(n.apps).join(", "),
    })),
    buses: tables(doc.bus).map((b) => ({
      id: str(b.id) ?? "?",
      type: str(b.type) ?? "?",
      protocol: str(b.protocol),
      connect: strs(b.connect),
      broker: str(b.broker),
    })),
  };
}

// ── Layout ────────────────────────────────────────────────────────────────────

export interface Placed<T> {
  item: T;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Edge {
  bus: string;
  node: string;
  iface: string;
  from: { x: number; y: number };
  to: { x: number; y: number };
}

export interface Layout {
  nodes: Placed<TNode>[];
  buses: Placed<TBus>[];
  edges: Edge[];
  width: number;
  height: number;
}

const NODE = { w: 180, h: 76 };
const BUS = { w: 150, h: 40 };
const GAP_X = 60;
const ROW = 190;
const PER_ROW = 4;

/**
 * Nodes in rows across the top, buses in a rail beneath, each bus centred under
 * the nodes it joins. A node-and-wire diagram this size needs no graph library,
 * and a layout that is a pure function of the declaration is a layout nobody
 * has to save.
 */
export function layout(t: Topo): Layout {
  const rows = Math.max(1, Math.ceil(t.nodes.length / PER_ROW));
  const nodes: Placed<TNode>[] = t.nodes.map((item, i) => ({
    item,
    x: 20 + (i % PER_ROW) * (NODE.w + GAP_X),
    y: 20 + Math.floor(i / PER_ROW) * ROW,
    ...NODE,
  }));
  const byId = new Map(nodes.map((n) => [n.item.id, n]));

  const railY = 20 + rows * ROW;
  const buses: Placed<TBus>[] = [];
  for (const bus of t.buses) {
    const ends = bus.connect.map((c) => byId.get(c.split(".")[0])).filter((n): n is Placed<TNode> => Boolean(n));
    const centre = ends.length ? ends.reduce((s, n) => s + n.x + n.w / 2, 0) / ends.length : 20 + BUS.w / 2;
    let x = centre - BUS.w / 2;
    // Two buses on the same row must not sit on top of each other.
    for (const other of buses) {
      if (Math.abs(other.x - x) < BUS.w + 20) x = other.x + BUS.w + 20;
    }
    buses.push({ item: bus, x: Math.max(20, x), y: railY, ...BUS });
  }

  // Each node's wires leave its bottom edge side by side, so their labels do
  // not pile up on the centre point.
  const perNode = new Map<string, number>();
  for (const b of buses) {
    for (const c of b.item.connect) {
      const id = c.split(".")[0];
      if (byId.has(id)) perNode.set(id, (perNode.get(id) ?? 0) + 1);
    }
  }
  const seen = new Map<string, number>();
  const edges: Edge[] = [];
  for (const b of buses) {
    for (const c of b.item.connect) {
      const [node, iface = ""] = c.split(".");
      const n = byId.get(node);
      if (!n) continue;
      const k = seen.get(node) ?? 0;
      seen.set(node, k + 1);
      const total = perNode.get(node) ?? 1;
      edges.push({
        bus: b.item.id,
        node,
        iface,
        from: { x: n.x + ((k + 1) / (total + 1)) * n.w, y: n.y + n.h },
        to: { x: b.x + b.w / 2, y: b.y },
      });
    }
  }

  const right = Math.max(...nodes.map((n) => n.x + n.w), ...buses.map((b) => b.x + b.w), 300);
  return { nodes, buses, edges, width: right + 20, height: railY + BUS.h + 30 };
}

// ── Edits: each returns new TOML text ─────────────────────────────────────────

/** The id an item will have if it is called `base`, made unique among `taken`. */
export function freshId(base: string, taken: string[]): string {
  if (!taken.includes(base)) return base;
  let n = 2;
  while (taken.includes(`${base}-${n}`)) n++;
  return `${base}-${n}`;
}

/**
 * A new node that already satisfies the declaration's rules: an MCU names the
 * firmware that runs on it, and every node belongs to a system. Adding a bare
 * `[[node]]` would be accepted by the editor and rejected by `fid derive`.
 */
export function addNode(text: string, doc: Doc, kind = "mcu"): { text: string; id: string } {
  const id = freshId("node", tables(doc.node).map((n) => str(n.id) ?? ""));
  let out = addBlock(text, "node", kind === "mcu" ? { id, kind, firmware: id } : { id, kind });
  const systems = tables(doc.system);
  if (systems.length > 0) out = editBlock(out, "system", 0, "nodes", [...strs(systems[0].nodes), id]);
  return { text: out, id };
}

export function addBus(text: string, doc: Doc, type = "i2c"): { text: string; id: string } {
  const id = freshId("bus", tables(doc.bus).map((b) => str(b.id) ?? ""));
  return { text: addBlock(text, "bus", { id, type, connect: [] }), id };
}

/**
 * Rename a node and every place that names it, so the declaration stays valid:
 * the systems that list it and the buses that connect to it or are brokered by
 * it. (A parameter's `scope` names the node's *firmware*, not its id, so a
 * rename leaves it alone; see `setFirmware`.) A rename that left a dangling
 * reference would turn into a `fid derive` error somewhere far from the cause.
 */
export function renameNode(text: string, doc: Doc, from: string, to: string): string {
  let out = text;
  tables(doc.system).forEach((s, i) => {
    const nodes = strs(s.nodes);
    if (nodes.includes(from)) out = editBlock(out, "system", i, "nodes", nodes.map((n) => (n === from ? to : n)));
  });
  tables(doc.bus).forEach((b, i) => {
    const connect = strs(b.connect);
    if (connect.some((c) => c.split(".")[0] === from)) {
      out = editBlock(out, "bus", i, "connect", connect.map((c) => (c.split(".")[0] === from ? `${to}${c.slice(from.length)}` : c)));
    }
    if (b.broker === from) out = editBlock(out, "bus", i, "broker", to);
  });
  const idx = tables(doc.node).findIndex((n) => n.id === from);
  return editBlock(out, "node", idx, "id", to);
}

/** Change a node's firmware, and the parameters scoped to the old name with it. */
export function setFirmware(text: string, doc: Doc, nodeId: string, to: string | undefined): string {
  const idx = tables(doc.node).findIndex((n) => n.id === nodeId);
  const from = str(tables(doc.node)[idx]?.firmware);
  let out = text;
  if (from && to) {
    tables(doc.parameter).forEach((p, i) => {
      if (p.scope === from) out = editBlock(out, "parameter", i, "scope", to);
    });
  }
  return editBlock(out, "node", idx, "firmware", to);
}

/** Rename a bus (nothing refers to a bus by id, so nothing else changes). */
export function renameBus(text: string, doc: Doc, from: string, to: string): string {
  const idx = tables(doc.bus).findIndex((b) => b.id === from);
  return editBlock(text, "bus", idx, "id", to);
}

/** Delete a node and the references to it, including parameters scoped to its firmware. */
export function removeNode(text: string, doc: Doc, id: string): string {
  let out = text;
  tables(doc.system).forEach((s, i) => {
    const nodes = strs(s.nodes);
    if (nodes.includes(id)) out = editBlock(out, "system", i, "nodes", nodes.filter((n) => n !== id));
  });
  tables(doc.bus).forEach((b, i) => {
    const connect = strs(b.connect);
    if (connect.some((c) => c.split(".")[0] === id)) {
      out = editBlock(out, "bus", i, "connect", connect.filter((c) => c.split(".")[0] !== id));
    }
    if (b.broker === id) out = editBlock(out, "bus", i, "broker", undefined);
  });
  // Blocks are removed last, highest index first, so earlier indices stay valid.
  const firmware = str(tables(doc.node).find((n) => n.id === id)?.firmware);
  const params = tables(doc.parameter)
    .map((p, i) => [p, i] as const)
    .filter(([p]) => firmware !== undefined && p.scope === firmware)
    .map(([, i]) => i)
    .reverse();
  for (const i of params) out = removeBlock(out, "parameter", i);
  const idx = tables(doc.node).findIndex((n) => n.id === id);
  return removeBlock(out, "node", idx);
}

export function removeBus(text: string, doc: Doc, id: string): string {
  return removeBlock(text, "bus", tables(doc.bus).findIndex((b) => b.id === id));
}

/** Wire `node.iface` onto a bus (a no-op if it already is). */
export function connect(text: string, doc: Doc, bus: string, endpoint: string): string {
  const i = tables(doc.bus).findIndex((b) => b.id === bus);
  const cur = strs(tables(doc.bus)[i]?.connect);
  return cur.includes(endpoint) ? text : editBlock(text, "bus", i, "connect", [...cur, endpoint]);
}

export function disconnect(text: string, doc: Doc, bus: string, endpoint: string): string {
  const i = tables(doc.bus).findIndex((b) => b.id === bus);
  return editBlock(text, "bus", i, "connect", strs(tables(doc.bus)[i]?.connect).filter((c) => c !== endpoint));
}

/** Every `node.interface` a bus could be wired to. */
export function endpoints(t: Topo): string[] {
  return t.nodes.flatMap((n) => n.interfaces.map((i) => `${n.id}.${i}`));
}

export { blockCount };


// ── The pipeline's outputs ────────────────────────────────────────────────────
//
// `fid derive` refuses a topology whose firmware has no listed output ("add to
// `outputs`"), because a pipeline's outputs are its contract with the freshness
// gate. So adding an MCU is a change to **two files**: the declaration and the
// list of files derived from it. The diagram makes both, so what it draws is
// something derive accepts.

/** `rs` for Rust (the default), `h` for C and C++, `json` for anything else. */
export function languageExt(language: string | undefined): "rs" | "h" | "json" {
  const l = (language ?? "rust").toLowerCase();
  if (l === "rust") return "rs";
  if (["c", "c++", "cpp", "cxx"].includes(l)) return "h";
  return "json";
}

const MANAGED = /^firmware\/[^/]+\/generated\/(params|topics)\.(rs|h|json)$/;

/** The outputs the topology's firmware requires, in a stable order. */
export function requiredOutputs(t: Topo): string[] {
  const out: string[] = [];
  for (const n of t.nodes) {
    if (!n.firmware) continue;
    const ext = languageExt(n.language);
    out.push(`firmware/${n.firmware}/generated/params.${ext}`, `firmware/${n.firmware}/generated/topics.${ext}`);
  }
  return [...new Set(out)];
}

/**
 * `pipelines/topology.toml` with its `outputs` matching the topology: the
 * per-firmware tables are added, removed or re-extensioned, and every other
 * output (a `board.rs`, a wiring diagram) is left exactly as it was.
 */
export function syncOutputs(pipelineText: string, t: Topo): string {
  const current = getPath(parseToml(pipelineText), ["outputs"]);
  const existing = Array.isArray(current) ? current.map(String) : [];
  const required = requiredOutputs(t);
  const kept = existing.filter((o) => !MANAGED.test(o));
  const managedBefore = existing.filter((o) => MANAGED.test(o));
  const same = managedBefore.length === required.length && required.every((r) => managedBefore.includes(r));
  if (same) return pipelineText;
  // Keep the original order of what survives; new entries go where the
  // managed ones were.
  const first = existing.findIndex((o) => MANAGED.test(o));
  const next = first === -1 ? [...required, ...kept] : [...kept.slice(0, first), ...required, ...kept.slice(first)];
  return editToml(pipelineText, ["outputs"], next);
}
