import { useEffect, useMemo, useState } from "react";
import { DndContext, DragOverlay, PointerSensor, KeyboardSensor, TouchSensor, closestCenter, useDraggable, useDroppable, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, Plus, Save, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { discoverBlocks, insertBlock, moveBlock, PageError, readPage, removeBlock, setAttr, type BlockDef, type Item, type Page } from "@/lib/blocks";
import { useSource } from "@/lib/source-context";

/** Pages Studio can offer: the app's route files that return JSX. */
const isPageFile = (f: string) => /(^|\/)apps\/[^/]+\/src\/(app|routes|pages)\/(.*\/)?(page|index)\.tsx$/.test(f);
/** Where a product keeps blocks: its own, and the design system's sections. */
const isBlockFile = (f: string) => /\/components\/(blocks\/[^/]+|ui\/sections)\.tsx$/.test(f);

/**
 * Compose a page from blocks by dragging them.
 *
 * What changes is the page's own source: a TSX file that returns a list of
 * components. Studio parses it, and every drag, drop and field edit patches
 * that text (see `lib/blocks.ts`), so the result is ordinary React the product
 * owns, reviewable as a diff, with nothing stored anywhere else.
 */
export default function Blocks({ onSaved }: { onSaved?: () => void }) {
  const source = useSource();
  const [pages, setPages] = useState<string[]>([]);
  const [catalog, setCatalog] = useState<BlockDef[]>([]);
  const [file, setFile] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [saved, setSaved] = useState("");
  const [sel, setSel] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);

  useEffect(() => {
    source.files().then(async (files) => {
      setPages(files.filter(isPageFile));
      const defs: BlockDef[] = [];
      for (const f of files.filter(isBlockFile)) {
        try {
          defs.push(...discoverBlocks(f, await source.read(f)));
        } catch { /* a file that cannot be read contributes no blocks */ }
      }
      setCatalog(defs);
    }).catch((e: Error) => setError(e.message));
  }, [source]);

  const open = (f: string) => {
    if (file && text !== saved && !confirm(`Discard unsaved changes to ${file}?`)) return;
    source.read(f).then((t) => { setFile(f); setText(t); setSaved(t); setSel(null); setError(null); }).catch((e: Error) => setError(e.message));
  };

  const parsed = useMemo<{ page: Page } | { error: string } | null>(() => {
    if (!file) return null;
    try {
      return { page: readPage(text) };
    } catch (e) {
      return { error: e instanceof PageError ? e.message : `This file does not parse: ${(e as Error).message}` };
    }
  }, [file, text]);
  const page = parsed && "page" in parsed ? parsed.page : null;
  const byName = useMemo(() => new Map(catalog.map((b) => [b.name, b])), [catalog]);

  /** Apply an edit; a refused one is a message, never a half-written file. */
  const apply = (fn: (t: string) => string, select?: number | null) => {
    try {
      const next = fn(text);
      readPage(next); // never hold a page that does not read back
      setText(next);
      setError(null);
      if (select !== undefined) setSel(select);
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const add = (def: BlockDef, at: number) => {
    // Required text props start filled in, so a new block is not blank on the page.
    const props: Record<string, string> = {};
    for (const p of def.props) if (p.required && p.type === "string") props[p.name] = p.name === "title" ? "New section" : p.name;
    apply((t) => insertBlock(t, at, def.name, def.importSource, props), at);
  };

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 150, tolerance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const onDragEnd = (e: DragEndEvent) => {
    setDragging(null);
    if (!page || !e.over) return;
    const id = String(e.active.id);
    const over = String(e.over.id);
    const overIndex = over.startsWith("item-") ? Number(over.slice(5)) : page.items.length;
    if (id.startsWith("palette:")) {
      const def = byName.get(id.slice(8));
      if (def) add(def, overIndex);
    } else if (id.startsWith("item-")) {
      const from = Number(id.slice(5));
      if (from !== overIndex && overIndex < page.items.length) apply((t) => moveBlock(t, from, overIndex), overIndex);
    }
  };

  const save = async () => {
    if (!file) return;
    try {
      await source.write(file, text, `studio: edit ${file}`);
      setSaved(text);
      onSaved?.();
      toast.success(source.review ? `Saved to ${source.review.branch()}` : `Saved ${file}`);
    } catch (e) {
      toast.error((e as Error).message);
    }
  };

  const dirty = file !== null && text !== saved;
  const selected = page && sel !== null ? page.items[sel] : undefined;
  const def = selected?.kind === "block" ? byName.get(selected.name) : undefined;

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragStart={(e) => setDragging(String(e.active.id))} onDragEnd={onDragEnd} onDragCancel={() => setDragging(null)}>
      <div className="grid gap-4 lg:grid-cols-[14rem_1fr_20rem]">
        <aside className="grid content-start gap-2">
          <h2 className="text-sm font-semibold">Blocks</h2>
          {catalog.length === 0 && <p className="text-xs text-muted-foreground">No blocks found. A product's blocks live in <code>components/blocks/</code>, and the design system's in <code>components/ui/sections.tsx</code>.</p>}
          <div className="grid gap-1.5">
            {catalog.map((b) => (
              <PaletteChip key={`${b.importSource}:${b.name}`} def={b} disabled={!page} onAdd={() => page && add(b, page.items.length)} />
            ))}
          </div>
        </aside>

        <div className="grid min-w-0 content-start gap-3">
          <PageBar pages={pages} file={file} onOpen={open} dirty={dirty} onSave={save} reviewing={Boolean(source.review)} kind={source.kind} />
          {error && <Alert variant="destructive"><AlertDescription>{error}</AlertDescription></Alert>}
          {parsed && "error" in parsed && <Alert><AlertDescription>{parsed.error}</AlertDescription></Alert>}
          {page && (
            <Canvas
              items={page.items}
              selected={sel}
              onSelect={setSel}
              onRemove={(i) => apply((t) => removeBlock(t, i), null)}
              hint={dragging?.startsWith("palette:")}
            />
          )}
          {!file && <p className="text-sm text-muted-foreground">Pick a page. Its blocks appear as a list you can reorder, add to and edit.</p>}
        </div>

        <aside className="grid content-start gap-3 rounded-lg border p-4">
          {!selected && <p className="text-sm text-muted-foreground">Select a block to edit its props.</p>}
          {selected && selected.kind === "opaque" && <p className="text-sm text-muted-foreground">{selected.name} is page code Studio moves but does not edit.</p>}
          {selected && selected.kind === "block" && sel !== null && (
            <PropPanel item={selected} def={def} onSet={(name, v) => apply((t) => setAttr(t, sel, name, v))} />
          )}
        </aside>
      </div>
      <DragOverlay>
        {dragging ? <div className="rounded-md border bg-card px-3 py-2 text-sm shadow">{dragging.startsWith("palette:") ? dragging.slice(8) : page?.items[Number(dragging.slice(5))]?.name}</div> : null}
      </DragOverlay>
    </DndContext>
  );
}

function PaletteChip({ def, disabled, onAdd }: { def: BlockDef; disabled: boolean; onAdd: () => void }) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: `palette:${def.name}`, disabled });
  return (
    <div ref={setNodeRef} className={`flex items-center gap-1 rounded-md border bg-card ${isDragging ? "opacity-40" : ""}`}>
      <button className="flex flex-1 cursor-grab items-center gap-2 px-2 py-1.5 text-left text-sm disabled:cursor-not-allowed disabled:opacity-50" disabled={disabled} {...attributes} {...listeners}>
        <GripVertical className="size-3.5 text-muted-foreground" /> {def.name}
      </button>
      <Button variant="ghost" size="icon-xs" aria-label={`Add ${def.name}`} disabled={disabled} onClick={onAdd}><Plus /></Button>
    </div>
  );
}

function PageBar({ pages, file, onOpen, dirty, onSave, reviewing, kind }: { pages: string[]; file: string | null; onOpen: (f: string) => void; dirty: boolean; onSave: () => void; reviewing: boolean; kind: string }) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Select value={file ?? undefined} onValueChange={onOpen}>
        <SelectTrigger className="min-w-60"><SelectValue placeholder={pages.length ? "Choose a page" : "No pages found"} /></SelectTrigger>
        <SelectContent>{pages.map((p) => <SelectItem key={p} value={p}>{p.replace(/^apps\/[^/]+\/src\//, "")}</SelectItem>)}</SelectContent>
      </Select>
      {dirty && <span className="text-xs text-muted-foreground">unsaved •</span>}
      <Button className="ml-auto" size="sm" onClick={onSave} disabled={!dirty}>
        <Save /> {reviewing ? "Save to branch" : kind === "github" ? "Commit" : "Save"}
      </Button>
    </div>
  );
}

function Canvas({ items, selected, onSelect, onRemove, hint }: { items: Item[]; selected: number | null; onSelect: (i: number) => void; onRemove: (i: number) => void; hint?: boolean }) {
  const { setNodeRef, isOver } = useDroppable({ id: "list" });
  const ids = items.map((_, i) => `item-${i}`);
  return (
    <div ref={setNodeRef} className={`grid min-h-40 content-start gap-2 rounded-lg border p-3 ${isOver || hint ? "border-primary/60" : ""}`}>
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        {items.map((it, i) => <Row key={ids[i]} id={ids[i]} item={it} active={selected === i} onSelect={() => onSelect(i)} onRemove={() => onRemove(i)} />)}
      </SortableContext>
      {items.length === 0 && <p className="text-sm text-muted-foreground">An empty page. Drag a block here, or press + beside one.</p>}
    </div>
  );
}

function Row({ id, item, active, onSelect, onRemove }: { id: string; item: Item; active: boolean; onSelect: () => void; onRemove: () => void }) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id });
  const summary = item.attrs.filter((a) => a.kind === "string").map((a) => String(a.value)).join(" · ");
  return (
    <div ref={setNodeRef} style={{ transform: CSS.Transform.toString(transform), transition }} className={`flex items-center gap-2 rounded-md border bg-card p-2 ${active ? "border-primary bg-accent" : ""}`} onClick={onSelect}>
      <button className="cursor-grab touch-none text-muted-foreground" aria-label={`Drag ${item.name}`} {...attributes} {...listeners}><GripVertical className="size-4" /></button>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2 text-sm font-medium">{item.name}{item.kind === "opaque" && <Badge variant="outline">code</Badge>}{item.hasChildren && item.kind === "block" && <Badge variant="secondary">has content</Badge>}</div>
        {summary && <p className="truncate text-xs text-muted-foreground">{summary}</p>}
      </div>
      <Button variant="ghost" size="icon-sm" aria-label={`Remove ${item.name}`} onClick={(e) => { e.stopPropagation(); onRemove(); }}><Trash2 /></Button>
    </div>
  );
}

function PropPanel({ item, def, onSet }: { item: Item; def?: BlockDef; onSet: (name: string, v: string | number | boolean | undefined) => void }) {
  const attr = (n: string) => item.attrs.find((a) => a.name === n);
  // Props the page sets but the component does not declare still show, as code.
  const known = new Set(def?.props.map((p) => p.name));
  const extra = item.attrs.filter((a) => !known.has(a.name));
  return (
    <>
      <h2 className="text-sm font-semibold">{item.name}</h2>
      {!def && <p className="text-xs text-muted-foreground">This component's props are not known (its file is not in the block catalog), so only the values already on the page are shown.</p>}
      {def?.props.map((p) => {
        const a = attr(p.name);
        const literal = a && a.kind !== "expression";
        return (
          <div key={p.name} className="grid gap-1.5">
            <Label className="font-mono text-xs">{p.name}{p.required && " *"}</Label>
            {a?.kind === "expression" ? (
              <code className="rounded bg-muted px-2 py-1 text-xs">{String(a.value)}</code>
            ) : p.type === "boolean" ? (
              <Switch checked={a?.value === true} onCheckedChange={(c) => onSet(p.name, c ? true : undefined)} />
            ) : p.type === "enum" ? (
              <Select value={literal ? String(a.value) : undefined} onValueChange={(v) => onSet(p.name, v)}>
                <SelectTrigger className="w-full"><SelectValue placeholder="not set" /></SelectTrigger>
                <SelectContent>{p.options!.map((o) => <SelectItem key={o} value={o}>{o}</SelectItem>)}</SelectContent>
              </Select>
            ) : p.type === "number" ? (
              <Input type="number" value={literal ? String(a.value) : ""} onChange={(e) => onSet(p.name, e.target.value === "" ? undefined : Number(e.target.value))} />
            ) : p.type === "string" ? (
              <Input value={literal ? String(a.value) : ""} onChange={(e) => onSet(p.name, e.target.value === "" ? undefined : e.target.value)} />
            ) : (
              <p className="text-xs text-muted-foreground">{a ? <code>{String(a.value)}</code> : "A code value: edit it in the file."}</p>
            )}
            {p.doc && <p className="text-xs text-muted-foreground">{p.doc}</p>}
          </div>
        );
      })}
      {extra.map((a) => (
        <div key={a.name} className="grid gap-1">
          <Label className="font-mono text-xs">{a.name}</Label>
          <code className="rounded bg-muted px-2 py-1 text-xs">{String(a.value)}</code>
        </div>
      ))}
    </>
  );
}
