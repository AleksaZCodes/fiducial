import { useEffect, useMemo, useState } from "react";
import { ExternalLink, PenLine } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { editorLink } from "@/lib/editor";
import { useSource } from "@/lib/source-context";
import { parseToml, type Json } from "@/lib/toml";
import type { DevStatus } from "@/lib/types";

/**
 * One click to the content editor the product declares.
 *
 * It reads `fiducial.toml` for which editor that is and works out where it is
 * reached (see `lib/editor.ts`); locally it starts the editor's own dev server
 * from here. Studio does not reimplement a CMS: Keystatic, Tina and Pages CMS
 * already are one, and this is only the door.
 */
export function ContentEditorCard() {
  const source = useSource();
  const [config, setConfig] = useState<Record<string, Json> | null>(null);
  const [dev, setDev] = useState<DevStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    source.read("fiducial.toml").then((t) => setConfig(parseToml(t))).catch(() => setConfig({}));
  }, [source]);

  // Follow the editor's dev server while one is starting.
  useEffect(() => {
    if (!source.dev) return;
    const poll = () => source.dev!.status("cms").then(setDev).catch(() => undefined);
    poll();
    const id = setInterval(poll, 1500);
    return () => clearInterval(id);
  }, [source]);

  const github = useMemo(() => {
    if (source.kind !== "github") return undefined;
    const [repo, branch] = source.label.split("@");
    const [owner, name] = repo.split("/");
    return { owner, repo: name, branch };
  }, [source]);

  const link = useMemo(
    () => (config ? editorLink({ config, github, devUrl: dev?.running ? dev.url : null, canStartDev: Boolean(source.dev) }) : null),
    [config, github, dev, source],
  );

  if (!link || (link.kind === "none" && link.reason.startsWith("This product has no"))) return null;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2"><PenLine className="size-4" /> Content</CardTitle>
        <CardDescription>The editor this product declares, over its own files</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-2 text-sm">
        {link.kind === "open" && (
          <Button asChild>
            <a href={link.url} target="_blank" rel="noreferrer"><ExternalLink /> {link.label}</a>
          </Button>
        )}
        {link.kind === "start" && (
          <Button
            onClick={() => source.dev!.start("cms").then(setDev).catch((e: Error) => setError(e.message))}
            disabled={Boolean(dev?.running)}
          >
            {dev?.running ? "Starting…" : link.label}
          </Button>
        )}
        {link.kind === "none" && <p className="text-muted-foreground">{link.reason}</p>}
        {error && <p className="text-destructive">{error}</p>}
      </CardContent>
    </Card>
  );
}
