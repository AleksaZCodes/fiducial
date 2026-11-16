import { useCallback, useEffect, useRef, useState } from "react";
import { ExternalLink, Laptop, Play, RefreshCw, Smartphone, Square, Tablet } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSource } from "@/lib/source-context";
import type { DevStatus } from "@/lib/types";

const VIEWPORTS = [
  { id: "phone", label: "Phone", width: 390, icon: Smartphone },
  { id: "tablet", label: "Tablet", width: 820, icon: Tablet },
  { id: "desktop", label: "Desktop", width: 0, icon: Laptop },
] as const;

/**
 * The product, running, inside Studio.
 *
 * With a local source the dev server is started from here and its URL is read
 * from its own output. With GitHub there is no machine to run one, so the pane
 * takes the URL of a deployed preview instead — the same iframe either way.
 *
 * An iframe can only show a page that allows being framed. A dev server does by
 * default; a deployed site that sends `X-Frame-Options: DENY` or a strict
 * `frame-ancestors` will show blank, and the pane says so instead of leaving
 * the person to wonder.
 */
export function Preview() {
  const source = useSource();
  const [dev, setDev] = useState<DevStatus | null>(null);
  const [custom, setCustom] = useState("");
  const [viewport, setViewport] = useState<(typeof VIEWPORTS)[number]["id"]>("desktop");
  const [nonce, setNonce] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const logEnd = useRef<HTMLDivElement>(null);

  const poll = useCallback(() => {
    source.dev?.status().then(setDev).catch((e: Error) => setError(e.message));
  }, [source]);
  useEffect(() => {
    poll();
    if (!source.dev) return;
    const id = setInterval(poll, 1500);
    return () => clearInterval(id);
  }, [poll, source]);
  useEffect(() => {
    logEnd.current?.scrollIntoView({ block: "end" });
  }, [dev?.log.length]);

  const url = custom || dev?.url || "";
  const width = VIEWPORTS.find((v) => v.id === viewport)!.width;

  return (
    <div className="grid h-[calc(100svh-6rem)] grid-rows-[auto_1fr] gap-3">
      <div className="flex flex-wrap items-center gap-2">
        {source.dev && (
          dev?.running ? (
            <Button variant="outline" onClick={() => source.dev!.stop().then(setDev)}><Square /> Stop</Button>
          ) : (
            <Button onClick={() => source.dev!.start().then(setDev).catch((e: Error) => setError(e.message))}><Play /> Start dev server</Button>
          )
        )}
        <Input
          className="min-w-48 flex-1"
          placeholder={source.dev ? (dev?.url ?? "dev server URL appears here") : "https://preview.example.com"}
          value={custom}
          onChange={(e) => setCustom(e.target.value)}
        />
        <div className="flex gap-1">
          {VIEWPORTS.map((v) => (
            <Button key={v.id} variant={viewport === v.id ? "secondary" : "ghost"} size="icon" aria-label={v.label} onClick={() => setViewport(v.id)}>
              <v.icon />
            </Button>
          ))}
        </div>
        <Button variant="ghost" size="icon" aria-label="Reload" onClick={() => setNonce((n) => n + 1)}><RefreshCw /></Button>
        {url && (
          <Button variant="ghost" size="icon" aria-label="Open in a new tab" asChild>
            <a href={url} target="_blank" rel="noreferrer"><ExternalLink /></a>
          </Button>
        )}
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="grid min-h-0 gap-3 lg:grid-cols-[1fr_20rem]">
        <div className="flex min-h-0 justify-center overflow-auto rounded-lg border bg-muted/40 p-2">
          {url ? (
            <iframe
              key={`${url}:${nonce}`}
              src={url}
              title="Preview"
              className="h-full rounded-md border bg-white"
              style={{ width: width ? `${width}px` : "100%" }}
              // Same-origin scripts are needed for a real app; the sandbox still
              // keeps it from navigating Studio or reading its storage.
              sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
            />
          ) : (
            <p className="self-center text-sm text-muted-foreground">
              {source.dev
                ? "Start the dev server, or paste a URL."
                : "Paste the URL of a deployed preview. A site that forbids framing will show blank."}
            </p>
          )}
        </div>
        {source.dev && (
          <ScrollArea className="hidden h-full rounded-lg border lg:block">
            <pre className="p-3 text-[11px] leading-relaxed text-muted-foreground">
              {dev?.command && `$ ${dev.command}\n`}
              {dev?.log.join("\n")}
              <div ref={logEnd} />
            </pre>
          </ScrollArea>
        )}
      </div>
    </div>
  );
}
