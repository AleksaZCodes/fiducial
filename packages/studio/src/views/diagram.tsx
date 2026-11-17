import { useEffect, useMemo, useState } from "react";
import { Cable, CheckCircle2, Plus, Save, Server, Trash2, X } from "lucide-react";
import { toast } from "sonner";
import { FieldView } from "@/components/schema-form";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { fieldOf, type Field } from "@/lib/schema";
import { useSource } from "@/lib/source-context";
import { getPath, parseToml, setPath, editBlock, type Json } from "@/lib/toml";
import * as T from "@/lib/topology";
import type { Schema } from "@/lib/types";

type Selection = { kind: "node" | "bus"; id: string } | null;

/** The declaration that holds the topology: the product's, not a capability's. */
export function findTopology(files: string[]): string | null {
  return files.find((f) => f === "topology.toml") ?? files.find((f) => /(^|\/)topology\.toml$/.test(f) && !f.includes("capabilities/")) ?? null;
}

/**
 * The system, drawn from `topology.toml` and edited by changing it.
 *
 * Nodes are boxes, buses are the rails beneath, each wire is an interface
 * (`node.iface`) on a bus. Click a thing to edit it; every change is a change
 * to the declaration text, comments preserved, and `fid derive` writes the
 * firmware tables, topics and pin maps from the same file. The positions are
 * computed, not stored: there is no second copy to drift.
 */
export function Diagram({ onSaved }: { onSaved?: () => void }) {
  const source = useSource();
  const [path, setPath_] = useState<string | null | undefined>(undefined);
  const [saved, setSaved] = useState("");
  const [text, setText] = useState("");
  const [schema, setSchema] = useState<Schema | null>(null);
  const [sel, setSel] = useState<Selection>(null);
  const [error, setError] = useState<string | null>(null);
  /** `pipelines/topology.toml`: which files are derived. Kept in step with the topology. */
  const [pipeBase, setPipeBase] = useState<string | null>(null);
  const [check, setCheck] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    source.files().then((f) => {
      const p = findTopology(f);
      setPath_(p);
      if (p) source.read(p).then((t) => (setText(t), setSaved(t))).catch((e: Error) => setError(e.message));
      const pipe = f.find((x) => x === "pipelines/topology.toml");
      if (pipe) source.read(pipe).then(setPipeBase).catch(() => setPipeBase(null));
    }).catch((e: Error) => setError(e.message));
    source.schema("topology").then(setSchema).catch(() => setSchema(null));
  }, [source]);

  const doc = useMemo(() => {
    try {
      return parseToml(text);
    } catch (e) {
      return null;
    }
  }, [text]);
  const topo = useMemo(() => (doc ? T.read(doc) : null), [doc]);
  const lay = useMemo(() => (topo ? T.layout(topo) : null), [topo]);
  // What the pipeline must list for derive to accept this topology.
  const pipeNext = useMemo(() => {
    if (pipeBase === null || !topo) return null;
    try {
      return T.syncOutputs(pipeBase, topo);
    } catch {
      return null;
    }
  }, [pipeBase, topo]);

  /** Apply an edit; a refused one is a message, never a half-written file. */
  const apply = (fn: (t: string, d: Record<string, Json>) => string, then?: Selection) => {
    if (!doc) return;
    try {
      const next = fn(text, doc);
      parseToml(next); // never hold text that does not parse
      setText(next);
      setError(null);
      if (then !== undefined) setSel(then);
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const save = async () => {
    if (!path) return;
    try {
      await source.write(path, text, "studio: edit topology");
      setSaved(text);
      if (pipeBase !== null && pipeNext !== null && pipeNext !== pipeBase) {
        await source.write("pipelines/topology.toml", pipeNext, "studio: keep the topology pipeline's outputs in step");
        setPipeBase(pipeNext);
      }
      onSaved?.();
      toast.success(source.review ? `Saved to ${source.review.branch()}` : "Saved topology.toml");
    } catch (e) {
      toast.error((e as Error).message);
    }
  };

  /** Run `fid derive` on what is saved. It enforces the declaration's own rules (an MCU needs firmware, every node belongs to a system, …) and writes what is derived from it. */
  const validate = async () => {
    try {
      const r = await source.derive!(false);
      setCheck({ ok: r.ok, text: `${r.stdout}\n${r.stderr}`.split("\n").filter((l) => /error|node|bus|scope|system|topology|stale/i.test(l) && !/^\s*\d+:|^\s*at /.test(l)).join("\n").trim() });
    } catch (e) {
      setCheck({ ok: false, text: (e as Error).message });
    }
  };

  if (path === undefined) return <p className="text-sm text-muted-foreground">Looking for the topology…</p>;
  if (path === null) {
    return (
      <Alert>
        <AlertDescription>
          This product has no <code>topology.toml</code>. Add the <code>topology</code> capability (<code>fid add topology</code>) and it appears here.
        </AlertDescription>
      </Alert>
    );
  }
  if (!doc || !topo || !lay) {
    return <Alert variant="destructive"><AlertDescription>topology.toml does not parse. Fix it in Declarations → Text first.</AlertDescription></Alert>;
  }

  const dirty = text !== saved || (pipeNext !== null && pipeBase !== null && pipeNext !== pipeBase);
  const selectedNode = sel?.kind === "node" ? topo.nodes.find((n) => n.id === sel.id) : undefined;
  const selectedBus = sel?.kind === "bus" ? topo.buses.find((b) => b.id === sel.id) : undefined;

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_22rem]">
      <div className="grid min-w-0 content-start gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => apply((t, d) => T.addNode(t, d).text, undefined)}>
            <Plus /> Node
          </Button>
          <Button variant="outline" size="sm" onClick={() => apply((t, d) => T.addBus(t, d).text, undefined)}>
            <Plus /> Bus
          </Button>
          <code className="text-xs text-muted-foreground">{path}{dirty && " •"}</code>
          {pipeNext !== null && pipeBase !== null && pipeNext !== pipeBase && (
            <span className="text-xs text-muted-foreground">+ pipelines/topology.toml outputs</span>
          )}
          {source.derive && (
            <Button className="ml-auto" variant="outline" size="sm" onClick={validate} disabled={dirty} title={dirty ? "Save first: this runs on the file on disk" : "Check the declaration's rules and write what is derived from it"}>
              <CheckCircle2 /> Derive
            </Button>
          )}
          <Button className={source.derive ? "" : "ml-auto"} size="sm" onClick={save} disabled={!dirty}>
            <Save /> {source.review ? "Save to branch" : source.kind === "github" ? "Commit" : "Save"}
          </Button>
        </div>
        {error && <Alert variant="destructive"><AlertDescription>{error}</AlertDescription></Alert>}
        {check && (
          <Alert variant={check.ok ? "default" : "destructive"}>
            <AlertDescription><pre className="whitespace-pre-wrap text-xs">{check.ok ? "Derived: the declaration holds together and its firmware tables, topics and pin maps are current." : check.text || "fid derive reported a problem."}</pre></AlertDescription>
          </Alert>
        )}

        <div className="overflow-auto rounded-lg border bg-card">
          <svg width={lay.width} height={lay.height} role="img" aria-label="System topology" className="block">
            {lay.edges.map((e, i) => {
              const mid = (e.from.y + e.to.y) / 2;
              return (
                <g key={`${e.bus}:${e.node}.${e.iface}:${i}`} className="text-muted-foreground">
                  <path d={`M${e.from.x},${e.from.y} C${e.from.x},${mid} ${e.to.x},${mid} ${e.to.x},${e.to.y}`} fill="none" stroke="currentColor" strokeWidth={1.5} />
                  <text x={e.from.x + 4} y={e.from.y + 14} fontSize={10} fill="currentColor">{e.iface}</text>
                </g>
              );
            })}
            {lay.buses.map(({ item, x, y, w, h }) => (
              <g key={item.id} onClick={() => setSel({ kind: "bus", id: item.id })} className="cursor-pointer">
                <rect x={x} y={y} width={w} height={h} rx={h / 2} className={sel?.id === item.id && sel.kind === "bus" ? "fill-accent stroke-primary" : "fill-secondary stroke-border"} strokeWidth={sel?.id === item.id && sel.kind === "bus" ? 2 : 1} />
                <text x={x + w / 2} y={y + h / 2 + 4} textAnchor="middle" fontSize={12} className="fill-foreground">{item.id} · {item.type}</text>
              </g>
            ))}
            {lay.nodes.map(({ item, x, y, w, h }) => (
              <g key={item.id} onClick={() => setSel({ kind: "node", id: item.id })} className="cursor-pointer">
                <rect x={x} y={y} width={w} height={h} rx={8} className={sel?.id === item.id && sel.kind === "node" ? "fill-accent stroke-primary" : "fill-card stroke-border"} strokeWidth={sel?.id === item.id && sel.kind === "node" ? 2 : 1} />
                <text x={x + 12} y={y + 24} fontSize={13} fontWeight={600} className="fill-foreground">{item.id}</text>
                <text x={x + 12} y={y + 42} fontSize={11} className="fill-muted-foreground">{item.kind}{item.detail ? ` · ${item.detail}` : ""}</text>
                <text x={x + 12} y={y + 60} fontSize={10} className="fill-muted-foreground">{item.interfaces.join("  ")}</text>
              </g>
            ))}
          </svg>
        </div>
        <p className="text-xs text-muted-foreground">
          Positions are computed from the declaration. Nothing is saved but <code>topology.toml</code>, and <code>fid derive</code> writes the firmware tables, topics and pin maps from it.
        </p>
      </div>

      <aside className="grid content-start gap-3 rounded-lg border p-4">
        {!selectedNode && !selectedBus && <p className="text-sm text-muted-foreground">Select a node or a bus.</p>}
        {selectedNode && schema && (
          <NodePanel
            node={selectedNode}
            index={topo.nodes.indexOf(selectedNode)}
            schema={schema}
            doc={doc}
            text={text}
            onRename={(to) => apply((t, d) => T.renameNode(t, d, selectedNode.id, to), { kind: "node", id: to })}
            onEdit={(i, key, v) =>
              apply((t, d) => (key === "firmware" ? T.setFirmware(t, d, selectedNode.id, v as string | undefined) : editBlock(t, "node", i, key, v)))
            }
            onDelete={() => apply((t, d) => T.removeNode(t, d, selectedNode.id), null)}
          />
        )}
        {selectedBus && schema && (
          <BusPanel
            bus={selectedBus}
            index={topo.buses.indexOf(selectedBus)}
            topo={topo}
            schema={schema}
            doc={doc}
            onRename={(to) => apply((t, d) => T.renameBus(t, d, selectedBus.id, to), { kind: "bus", id: to })}
            onEdit={(i, key, v) => apply((t) => editBlock(t, "bus", i, key, v))}
            onConnect={(ep) => apply((t, d) => T.connect(t, d, selectedBus.id, ep))}
            onDisconnect={(ep) => apply((t, d) => T.disconnect(t, d, selectedBus.id, ep))}
            onDelete={() => apply((t, d) => T.removeBus(t, d, selectedBus.id), null)}
          />
        )}
      </aside>
    </div>
  );
}

/** The form for one block, from the schema's definition of that block. */
function itemField(schema: Schema, def: "Node" | "Bus", skip: string[]): Extract<Field, { kind: "object" }> | null {
  const f = fieldOf(schema, { $ref: `#/$defs/${def}` });
  return f.kind === "object" ? { ...f, props: f.props.filter((p) => !skip.includes(p.name)) } : null;
}

function IdField({ value, onCommit }: { value: string; onCommit: (v: string) => void }) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  return (
    <div className="grid gap-1.5">
      <Label className="font-mono text-xs">id</Label>
      {/* Committed on blur: a rename rewrites every reference, so it must not
          run once per keystroke. */}
      <Input value={draft} onChange={(e) => setDraft(e.target.value)} onBlur={() => draft.trim() && draft !== value && onCommit(draft.trim())} />
    </div>
  );
}

function ItemForm({ field, item, index, onEdit }: { field: Extract<Field, { kind: "object" }>; item: Record<string, Json>; index: number; onEdit: (i: number, key: string, v: Json | undefined) => void }) {
  const doc = { item } as Record<string, Json>;
  return (
    <div className="grid gap-4">
      {field.props.map(({ name, field: f }) => (
        <div key={name} className="grid gap-1.5">
          <Label className="font-mono text-xs">{name}</Label>
          <FieldView
            field={f}
            path={["item", name]}
            doc={doc}
            onChange={(p, v) => {
              // A nested change (`rt.priority`) rewrites that key as a whole.
              const next = setPath(doc, p, v as Json);
              onEdit(index, name, getPath(next, ["item", name]));
            }}
          />
        </div>
      ))}
    </div>
  );
}

function NodePanel({ node, index, schema, doc, onRename, onEdit, onDelete }: { node: T.TNode; index: number; schema: Schema; doc: Record<string, Json>; text: string; onRename: (to: string) => void; onEdit: (i: number, k: string, v: Json | undefined) => void; onDelete: () => void }) {
  const field = itemField(schema, "Node", ["id"]);
  const item = ((doc.node as Record<string, Json>[] | undefined) ?? [])[index] ?? {};
  return (
    <>
      <div className="flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-sm font-semibold"><Server className="size-4" /> Node</h2>
        <Button variant="ghost" size="icon-sm" aria-label="Delete node" onClick={onDelete}><Trash2 /></Button>
      </div>
      <IdField value={node.id} onCommit={onRename} />
      {field && <ItemForm field={field} item={item} index={index} onEdit={onEdit} />}
    </>
  );
}

function BusPanel({ bus, index, topo, schema, doc, onRename, onEdit, onConnect, onDisconnect, onDelete }: { bus: T.TBus; index: number; topo: T.Topo; schema: Schema; doc: Record<string, Json>; onRename: (to: string) => void; onEdit: (i: number, k: string, v: Json | undefined) => void; onConnect: (ep: string) => void; onDisconnect: (ep: string) => void; onDelete: () => void }) {
  // `connect` has its own control below; the rest comes from the schema.
  const field = itemField(schema, "Bus", ["id", "connect"]);
  const item = ((doc.bus as Record<string, Json>[] | undefined) ?? [])[index] ?? {};
  const free = T.endpoints(topo).filter((e) => !bus.connect.includes(e));
  return (
    <>
      <div className="flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-sm font-semibold"><Cable className="size-4" /> Bus</h2>
        <Button variant="ghost" size="icon-sm" aria-label="Delete bus" onClick={onDelete}><Trash2 /></Button>
      </div>
      <IdField value={bus.id} onCommit={onRename} />
      <div className="grid gap-1.5">
        <Label className="font-mono text-xs">connect</Label>
        <ul className="grid gap-1">
          {bus.connect.map((c) => (
            <li key={c} className="flex items-center justify-between rounded-md border px-2 py-1 text-xs">
              <code>{c}</code>
              <Button variant="ghost" size="icon-xs" aria-label={`Disconnect ${c}`} onClick={() => onDisconnect(c)}><X /></Button>
            </li>
          ))}
        </ul>
        {free.length > 0 && (
          <Select value="" onValueChange={onConnect}>
            <SelectTrigger className="w-full"><SelectValue placeholder="Connect an interface…" /></SelectTrigger>
            <SelectContent>{free.map((e) => <SelectItem key={e} value={e}>{e}</SelectItem>)}</SelectContent>
          </Select>
        )}
      </div>
      {field && <ItemForm field={field} item={item} index={index} onEdit={onEdit} />}
    </>
  );
}
