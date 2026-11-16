import { useEffect, useMemo, useState } from "react";
import { FileText, Save, Search } from "lucide-react";
import { toast } from "sonner";
import { SchemaForm } from "@/components/schema-form";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useSource } from "@/lib/source-context";
import { EditRefused, editToml, parseToml, type Json } from "@/lib/toml";
import type { Schema } from "@/lib/types";

/** Declarations first: the files a person edits to change what is derived. */
function rank(path: string): number {
  if (path === "fiducial.toml") return 0;
  if (/(^|\/)declarations\//.test(path) || /design-system\.md$|topology\.toml$/.test(path)) return 1;
  if (/\.toml$/.test(path)) return 2;
  return 3;
}

export function Declarations({ onSaved }: { onSaved?: () => void }) {
  const source = useSource();
  const [files, setFiles] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<string | null>(null);
  const [saved, setSaved] = useState("");
  const [text, setText] = useState("");
  const [schema, setSchema] = useState<Schema | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    source.files().then(setFiles).catch((e: Error) => setProblem(e.message));
    source.schema().then(setSchema).catch(() => setSchema(null));
  }, [source]);

  const shown = useMemo(
    () =>
      files
        .filter((f) => f.toLowerCase().includes(query.toLowerCase()))
        .sort((a, b) => rank(a) - rank(b) || a.localeCompare(b))
        .slice(0, 300),
    [files, query],
  );

  const load = (path: string) => {
    if (open && text !== saved && !confirm(`Discard unsaved changes to ${open}?`)) return;
    setProblem(null);
    source.read(path).then((t) => {
      setOpen(path);
      setText(t);
      setSaved(t);
    }).catch((e: Error) => setProblem(e.message));
  };

  const save = async () => {
    if (!open) return;
    setSaving(true);
    try {
      await source.write(open, text);
      setSaved(text);
      onSaved?.();
      toast.success(source.review ? `Saved ${open} to ${source.review.branch()}` : source.kind === "github" ? `Committed ${open}` : `Saved ${open}`);
    } catch (e) {
      toast.error((e as Error).message);
    } finally {
      setSaving(false);
    }
  };

  const doc = useMemo(() => {
    if (open !== "fiducial.toml") return null;
    try {
      return parseToml(text);
    } catch {
      return null;
    }
  }, [open, text]);

  const edit = (path: string[], value: Json | undefined) => {
    try {
      setText((t) => editToml(t, path, value));
      setProblem(null);
    } catch (e) {
      setProblem(e instanceof EditRefused ? `${e.message} — edit this one in the Text tab.` : (e as Error).message);
    }
  };

  const dirty = open !== null && text !== saved;

  return (
    <div className="grid h-[calc(100svh-6rem)] gap-4 md:grid-cols-[18rem_1fr]">
      <div className="flex min-h-0 flex-col gap-2">
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
          <Input className="pl-8" placeholder="Find a file" value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
        <ScrollArea className="min-h-0 flex-1 rounded-lg border">
          <ul className="p-1">
            {shown.map((f) => (
              <li key={f}>
                <button
                  onClick={() => load(f)}
                  className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs hover:bg-accent ${f === open ? "bg-accent" : ""}`}
                >
                  <FileText className="size-3.5 shrink-0 text-muted-foreground" />
                  <span className="truncate font-mono">{f}</span>
                  {rank(f) < 2 && <Badge variant="secondary" className="ml-auto shrink-0">declaration</Badge>}
                </button>
              </li>
            ))}
          </ul>
        </ScrollArea>
      </div>

      <div className="flex min-h-0 flex-col gap-3">
        {problem && (
          <Alert variant="destructive"><AlertDescription>{problem}</AlertDescription></Alert>
        )}
        {!open ? (
          <p className="text-sm text-muted-foreground">Pick a file. <code>fiducial.toml</code> opens as a form.</p>
        ) : (
          <>
            <div className="flex items-center justify-between gap-2">
              <code className="truncate text-sm">{open}{dirty && " •"}</code>
              <Button onClick={save} disabled={!dirty || saving}>
                <Save /> {source.review ? "Save to branch" : source.kind === "github" ? "Commit" : "Save"}
              </Button>
            </div>
            <Tabs defaultValue={doc && schema ? "form" : "text"} key={open} className="min-h-0 flex-1">
              <TabsList>
                <TabsTrigger value="form" disabled={!doc || !schema}>Form</TabsTrigger>
                <TabsTrigger value="text">Text</TabsTrigger>
              </TabsList>
              <TabsContent value="form" className="min-h-0">
                <ScrollArea className="h-[calc(100svh-14rem)] pr-3">
                  {doc && schema && <SchemaForm schema={schema} doc={doc} onChange={edit} />}
                </ScrollArea>
              </TabsContent>
              <TabsContent value="text" className="min-h-0">
                <Textarea
                  spellCheck={false}
                  className="h-[calc(100svh-14rem)] resize-none font-mono text-xs"
                  value={text}
                  onChange={(e) => setText(e.target.value)}
                />
              </TabsContent>
            </Tabs>
          </>
        )}
      </div>
    </div>
  );
}
