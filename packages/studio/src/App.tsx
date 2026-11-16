import { useEffect, useMemo, useState } from "react";
import { Eye, FileCog, LayoutDashboard, LogOut } from "lucide-react";
import { ReviewBar } from "@/components/review-bar";
import { Toaster } from "@/components/ui/sonner";
import {
  Sidebar, SidebarContent, SidebarFooter, SidebarGroup, SidebarHeader, SidebarInset, SidebarMenu,
  SidebarMenuButton, SidebarMenuItem, SidebarProvider, SidebarTrigger,
} from "@/components/ui/sidebar";
import { loadConfig, type StudioConfig } from "@/lib/config";
import { SourceContext } from "@/lib/source-context";
import { githubSource, type GithubTarget } from "@/lib/sources/github";
import { localSource } from "@/lib/sources/local";
import type { Schema, Source } from "@/lib/types";
import schema from "@/fiducial.schema.json";
import { Connect } from "@/views/connect";
import { Dashboard } from "@/views/dashboard";
import { Declarations } from "@/views/declarations";
import { Preview } from "@/views/preview";

type View = "dashboard" | "declarations" | "preview";
const NAV: { id: View; label: string; icon: typeof Eye }[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "declarations", label: "Declarations", icon: FileCog },
  { id: "preview", label: "Preview", icon: Eye },
];

const KEY = "studio.github";

function safe<T>(fn: () => T): T | null {
  try {
    return fn();
  } catch {
    return null;
  }
}

/** A token GitHub sign-in just returned in the URL fragment, or one kept for this tab. */
function signedInToken(): string | null {
  const hash = new URLSearchParams(location.hash.slice(1));
  const fresh = hash.get("gh_token");
  if (fresh) {
    safe(() => sessionStorage.setItem("studio.ghtoken", fresh));
    history.replaceState(null, "", location.pathname + location.search);
    return fresh;
  }
  return safe(() => sessionStorage.getItem("studio.ghtoken"));
}

/** Pick the source from how Studio was opened. `fid studio` prints a URL with
 *  `#token=…`; the fragment never goes to a server, and is moved to session
 *  storage so a reload keeps working. */
function initialSource(): Source | null {
  const hash = new URLSearchParams(location.hash.slice(1));
  const params = new URLSearchParams(location.search);
  const token = hash.get("token") ?? safe(() => sessionStorage.getItem("studio.token"));
  if (token) {
    safe(() => sessionStorage.setItem("studio.token", token));
    history.replaceState(null, "", location.pathname + location.search);
    return localSource(params.get("api") ?? "", token);
  }
  const saved = safe(() => localStorage.getItem(KEY) ?? sessionStorage.getItem(KEY));
  if (saved) return githubSource(JSON.parse(saved) as GithubTarget, schema as Schema);
  return null;
}

export default function App() {
  const [source, setSource] = useState<Source | null>(initialSource);
  const [view, setView] = useState<View>("dashboard");
  const [saves, setSaves] = useState(0);
  const [config, setConfig] = useState<StudioConfig>({ authUrl: "" });
  const [token] = useState(signedInToken);
  useEffect(() => {
    loadConfig().then(setConfig);
  }, []);
  useEffect(() => {
    document.title = source ? `${source.label} · Studio` : "Studio";
  }, [source]);
  const body = useMemo(
    () => ({ dashboard: <Dashboard />, declarations: <Declarations onSaved={() => setSaves((n) => n + 1)} />, preview: <Preview /> })[view],
    [view],
  );

  if (!source) {
    return (
      <>
        <Connect
          authUrl={config.authUrl}
          signedInToken={token}
          onGithub={(t, remember) => {
            safe(() => (remember ? localStorage : sessionStorage).setItem(KEY, JSON.stringify(t)));
            setSource(githubSource(t, schema as Schema));
          }}
        />
        <Toaster />
      </>
    );
  }

  return (
    <SourceContext.Provider value={source}>
      <SidebarProvider>
        <Sidebar>
          <SidebarHeader className="px-4 py-3">
            <strong>Studio</strong>
            <span className="truncate text-xs text-muted-foreground">{source.label}</span>
          </SidebarHeader>
          <SidebarContent>
            <SidebarGroup>
              <SidebarMenu>
                {NAV.map((n) => (
                  <SidebarMenuItem key={n.id}>
                    <SidebarMenuButton isActive={view === n.id} onClick={() => setView(n.id)}>
                      <n.icon /> {n.label}
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroup>
          </SidebarContent>
          <SidebarFooter>
            {source.kind === "github" && (
              <SidebarMenuButton
                onClick={() => {
                  safe(() => { localStorage.removeItem(KEY); sessionStorage.removeItem(KEY); sessionStorage.removeItem("studio.ghtoken"); });
                  setSource(null);
                }}
              >
                <LogOut /> Disconnect
              </SidebarMenuButton>
            )}
          </SidebarFooter>
        </Sidebar>
        <SidebarInset>
          <header className="flex h-12 items-center gap-2 border-b px-3">
            <SidebarTrigger />
            <span className="text-sm font-medium">{NAV.find((n) => n.id === view)!.label}</span>
            <div className="ml-auto"><ReviewBar version={saves} /></div>
          </header>
          <main className="p-4">{body}</main>
        </SidebarInset>
      </SidebarProvider>
      <Toaster />
    </SourceContext.Provider>
  );
}
