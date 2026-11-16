import { useMemo, useState } from "react";
import { Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { fieldOf, summary, type Field } from "@/lib/schema";
import type { Schema } from "@/lib/types";
import { getPath, type Json } from "@/lib/toml";

/** Called with a path and the new value, or `undefined` to remove the key. */
export type OnChange = (path: string[], value: Json | undefined) => void;

/**
 * A form for a TOML document, generated from its JSON Schema. The document is
 * the source of truth and the form is a view of it: every control reports a
 * path and a value, and the caller patches the text, so comments survive.
 */
export function SchemaForm({ schema, doc, onChange }: { schema: Schema; doc: Record<string, Json>; onChange: OnChange }) {
  const root = useMemo(() => fieldOf(schema, schema), [schema]);
  if (root.kind !== "object") return null;
  return (
    <div className="flex flex-col gap-3">
      {root.props.map(({ name, field }) => (
        <Section key={name} name={name} field={field} path={[name]} doc={doc} onChange={onChange} />
      ))}
    </div>
  );
}

function Section({ name, field, path, doc, onChange }: Props & { name: string }) {
  const present = getPath(doc, path) !== undefined;
  const [open, setOpen] = useState(present);
  return (
    <details
      className="rounded-lg border bg-card"
      open={open}
      onToggle={(e) => setOpen((e.target as HTMLDetailsElement).open)}
    >
      <summary className="flex cursor-pointer items-center gap-2 px-4 py-3 text-sm font-medium">
        <code>{name}</code>
        {!present && <span className="text-xs font-normal text-muted-foreground">not declared</span>}
        <span className="ml-2 line-clamp-1 text-xs font-normal text-muted-foreground">{summary(field.description)}</span>
      </summary>
      <div className="border-t px-4 py-3">
        {open && <FieldView field={field} path={path} doc={doc} onChange={onChange} />}
      </div>
    </details>
  );
}

interface Props {
  field: Field;
  path: string[];
  doc: Record<string, Json>;
  onChange: OnChange;
}

function Labelled({ name, hint, children }: { name: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <Label className="font-mono text-xs">{name}</Label>
      {children}
      {hint && <p className="text-xs text-muted-foreground">{summary(hint)}</p>}
    </div>
  );
}

function FieldView({ field, path, doc, onChange }: Props) {
  const value = getPath(doc, path);
  switch (field.kind) {
    case "object":
      return (
        <div className="grid gap-4">
          {field.props.map(({ name, field: f }) =>
            f.kind === "object" || f.kind === "map" ? (
              <fieldset key={name} className="grid gap-2 rounded-md border p-3">
                <legend className="px-1 font-mono text-xs">{name}</legend>
                <FieldView field={f} path={[...path, name]} doc={doc} onChange={onChange} />
              </fieldset>
            ) : (
              <Labelled key={name} name={name} hint={f.description}>
                <FieldView field={f} path={[...path, name]} doc={doc} onChange={onChange} />
              </Labelled>
            ),
          )}
        </div>
      );
    case "string":
      return (
        <Input
          value={typeof value === "string" ? value : ""}
          onChange={(e) => onChange(path, e.target.value === "" ? undefined : e.target.value)}
        />
      );
    case "number":
      return (
        <Input
          type="number"
          value={typeof value === "number" ? value : ""}
          onChange={(e) => onChange(path, e.target.value === "" ? undefined : Number(e.target.value))}
        />
      );
    case "boolean":
      return <Switch checked={value === true} onCheckedChange={(c) => onChange(path, c)} />;
    case "enum":
      return (
        <Select value={typeof value === "string" ? value : undefined} onValueChange={(v) => onChange(path, v)}>
          <SelectTrigger className="w-full"><SelectValue placeholder="not set" /></SelectTrigger>
          <SelectContent>
            {field.options.map((o) => <SelectItem key={o} value={o}>{o}</SelectItem>)}
          </SelectContent>
        </Select>
      );
    case "list":
      if (field.item.kind === "string" || field.item.kind === "enum") {
        const items = Array.isArray(value) ? value.map(String) : [];
        return (
          <Input
            placeholder="comma-separated"
            value={items.join(", ")}
            onChange={(e) => {
              const next = e.target.value.split(",").map((s) => s.trim()).filter(Boolean);
              onChange(path, next.length ? next : undefined);
            }}
          />
        );
      }
      return <JsonBox path={path} value={value} onChange={onChange} />;
    case "map":
      return <MapView field={field} path={path} doc={doc} onChange={onChange} />;
    default:
      return <JsonBox path={path} value={value} onChange={onChange} />;
  }
}

function MapView({ field, path, doc, onChange }: Props & { field: Extract<Field, { kind: "map" }> }) {
  const value = getPath(doc, path);
  const entries = value && typeof value === "object" && !Array.isArray(value) ? Object.keys(value) : [];
  const [draft, setDraft] = useState("");
  return (
    <div className="grid gap-3">
      {entries.map((k) => (
        <div key={k} className="grid gap-1.5">
          <div className="flex items-center justify-between">
            <Label className="font-mono text-xs">{k}</Label>
            <Button variant="ghost" size="icon-sm" aria-label={`remove ${k}`} onClick={() => onChange([...path, k], undefined)}>
              <X />
            </Button>
          </div>
          <FieldView field={field.value} path={[...path, k]} doc={doc} onChange={onChange} />
        </div>
      ))}
      <div className="flex gap-2">
        <Input placeholder="new entry name" value={draft} onChange={(e) => setDraft(e.target.value)} />
        <Button
          variant="outline"
          disabled={!draft || entries.includes(draft)}
          onClick={() => {
            onChange([...path, draft], field.value.kind === "object" || field.value.kind === "map" ? {} : "");
            setDraft("");
          }}
        >
          <Plus /> Add
        </Button>
      </div>
    </div>
  );
}

/** The fallback: a value Studio has no control for is still editable, as JSON. */
function JsonBox({ path, value, onChange }: { path: string[]; value: Json | undefined; onChange: OnChange }) {
  const [text, setText] = useState(value === undefined ? "" : JSON.stringify(value));
  const [bad, setBad] = useState(false);
  return (
    <div className="grid gap-1">
      <Textarea
        className="font-mono text-xs"
        value={text}
        aria-invalid={bad}
        onChange={(e) => {
          setText(e.target.value);
          if (e.target.value.trim() === "") return onChange(path, undefined), setBad(false);
          try {
            onChange(path, JSON.parse(e.target.value) as Json);
            setBad(false);
          } catch {
            setBad(true);
          }
        }}
      />
      {bad && <p className="text-xs text-destructive">Not valid JSON yet — nothing is saved until it is.</p>}
    </div>
  );
}
