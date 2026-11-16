import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, GitBranch, RefreshCw } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useSource } from "@/lib/source-context";
import type { Dash } from "@/lib/types";

export function Dashboard() {
  const source = useSource();
  const [dash, setDash] = useState<Dash | null>(null);
  const [files, setFiles] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [derive, setDerive] = useState<string | null>(null);

  const load = useCallback(() => {
    setError(null);
    source.files().then(setFiles).catch((e: Error) => setError(e.message));
    source.dash?.().then(setDash).catch((e: Error) => setError(e.message));
  }, [source]);
  useEffect(load, [load]);

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertTriangle />
        <AlertTitle>Could not read the product</AlertTitle>
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  const fresh = dash && dash.freshness.artifacts_stale + dash.freshness.artifacts_missing === 0 && dash.freshness.problems.length === 0;

  return (
    <div className="grid gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">{dash?.product.name ?? source.label}</h1>
          <p className="text-sm text-muted-foreground">
            {dash ? `v${dash.product.version} · ${dash.product.capabilities.length} capabilities` : source.kind === "github" ? "Editing through GitHub" : "Reading the product…"}
          </p>
        </div>
        <div className="flex gap-2">
          {source.derive && (
            <Button
              variant="outline"
              onClick={() => {
                setDerive("running…");
                source.derive!(true).then((r) => {
                  setDerive(r.ok ? "Everything derived is current." : `${r.stdout}${r.stderr}`.trim());
                  load();
                }).catch((e: Error) => setDerive(e.message));
              }}
            >
              Check derived files
            </Button>
          )}
          <Button variant="outline" size="icon" aria-label="Refresh" onClick={load}><RefreshCw /></Button>
        </div>
      </div>

      {derive && (
        <pre className="max-h-48 overflow-auto rounded-lg border bg-muted p-3 text-xs">{derive}</pre>
      )}

      {!source.dash && (
        <Alert>
          <AlertTitle>No dashboard over GitHub</AlertTitle>
          <AlertDescription>
            Freshness needs <code>fid</code>, which runs on a machine or in CI. Edit here and commit; CI reports what is
            stale. Run <code>fid studio</code> locally for the full view.
          </AlertDescription>
        </Alert>
      )}

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle>Freshness</CardTitle>
            <CardDescription>Generated files against their declarations</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-2 text-sm">
            {!dash ? <Skeleton className="h-16" /> : (
              <>
                <div className="flex items-center gap-2">
                  {fresh ? <CheckCircle2 className="size-4 text-green-600" /> : <AlertTriangle className="size-4 text-destructive" />}
                  <span>{fresh ? "All current" : "Needs `fid derive`"}</span>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Badge variant="secondary">{dash.freshness.artifacts_fresh} fresh</Badge>
                  {dash.freshness.artifacts_stale > 0 && <Badge variant="destructive">{dash.freshness.artifacts_stale} stale</Badge>}
                  {dash.freshness.artifacts_missing > 0 && <Badge variant="destructive">{dash.freshness.artifacts_missing} missing</Badge>}
                </div>
                {dash.freshness.problems.map((p) => <p key={p} className="text-xs text-muted-foreground">{p}</p>)}
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Repository</CardTitle>
            <CardDescription>Where this working copy stands</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-2 text-sm">
            {!dash ? <Skeleton className="h-16" /> : (
              <>
                <div className="flex items-center gap-2"><GitBranch className="size-4" />{dash.git.branch ?? "no branch"}<code className="text-xs text-muted-foreground">{dash.git.commit}</code></div>
                <p className="line-clamp-2 text-muted-foreground">{dash.git.subject}</p>
                <div className="flex gap-2">
                  {dash.git.dirty_files > 0 && <Badge variant="outline">{dash.git.dirty_files} changed</Badge>}
                  {dash.git.ahead > 0 && <Badge variant="outline">{dash.git.ahead} ahead</Badge>}
                  {dash.git.behind > 0 && <Badge variant="outline">{dash.git.behind} behind</Badge>}
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Roadmap</CardTitle>
            <CardDescription>From the product's own roadmap file</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-2 text-sm">
            {!dash ? <Skeleton className="h-16" /> : (
              <>
                <div className="flex gap-2">
                  <Badge>{dash.roadmap.done} done</Badge>
                  <Badge variant="secondary">{dash.roadmap.in_progress} in progress</Badge>
                  <Badge variant="outline">{dash.roadmap.todo} to do</Badge>
                </div>
                {dash.roadmap.next && <p className="text-muted-foreground">Next: {dash.roadmap.next}</p>}
              </>
            )}
          </CardContent>
        </Card>

        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Declarations</CardTitle>
            <CardDescription>The facts each capability reads, and whether they are written</CardDescription>
          </CardHeader>
          <CardContent>
            {!dash ? <Skeleton className="h-16" /> : (
              <ul className="grid gap-1 text-sm sm:grid-cols-2">
                {dash.taxonomy.declarations.map((d) => (
                  <li key={`${d.capability}:${d.name}`} className="flex items-center justify-between gap-2">
                    <code className="truncate text-xs">{d.name}</code>
                    <Badge variant={d.present ? "secondary" : "destructive"}>{d.present ? "present" : "missing"}</Badge>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{files ? `${files.length} editable files` : "Files"}</CardTitle>
            <CardDescription>Everything Studio can open</CardDescription>
          </CardHeader>
          <CardContent>{!files && <Skeleton className="h-8" />}</CardContent>
        </Card>

        {dash && dash.graph.pipelines.length > 0 && (
          <Card className="md:col-span-2 xl:col-span-3">
            <CardHeader>
              <CardTitle>Pipelines</CardTitle>
              <CardDescription>What derives what</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-2 text-sm md:grid-cols-2">
              {dash.graph.pipelines.map((p) => (
                <div key={p.name} className="rounded-md border p-3">
                  <div className="flex items-center justify-between"><strong>{p.name}</strong><code className="text-xs">{p.executor}</code></div>
                  <ul className="mt-1 text-xs text-muted-foreground">{p.outputs.map((o) => <li key={o}>{o}</li>)}</ul>
                </div>
              ))}
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
