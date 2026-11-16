import { useState } from "react";
import { GitBranch, HardDrive } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { signInUrl } from "@/lib/config";
import type { GithubTarget } from "@/lib/sources/github";

/**
 * The first screen when Studio was not opened by `fid studio`.
 *
 * GitHub sign-in is a small Worker (`worker/`) that holds the OAuth client
 * secret, because a browser cannot finish GitHub's web flow on its own. Without
 * one configured, a pasted fine-grained token is the working path. Either way
 * the token stays in this browser and goes only to api.github.com.
 */
export function Connect({
  onGithub,
  authUrl,
  signedInToken,
}: {
  onGithub: (t: GithubTarget, remember: boolean) => void;
  /** The sign-in Worker, when this deployment has one. */
  authUrl: string;
  /** A token the sign-in just returned. */
  signedInToken: string | null;
}) {
  const [repo, setRepo] = useState("");
  const [branch, setBranch] = useState("main");
  const [mode, setMode] = useState<"commit" | "pr">("pr");
  const [typed, setToken] = useState("");
  const token = signedInToken ?? typed;
  const [remember, setRemember] = useState(false);
  const [owner, name] = repo.split("/");
  const ok = Boolean(owner && name && token && branch);

  return (
    <div className="mx-auto grid max-w-xl gap-4 p-4 pt-12">
      <div>
        <h1 className="text-2xl font-semibold">Studio</h1>
        <p className="text-sm text-muted-foreground">Edit a product's declarations. Choose where it lives.</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2"><GitBranch className="size-4" /> A GitHub repository</CardTitle>
          <CardDescription>Works from any browser, including a phone. Saves land as a pull request, or as commits.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3">
          <div className="grid gap-1.5">
            <Label htmlFor="repo">Repository</Label>
            <Input id="repo" placeholder="owner/name" value={repo} onChange={(e) => setRepo(e.target.value.trim())} />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="branch">Branch</Label>
            <Input id="branch" value={branch} onChange={(e) => setBranch(e.target.value.trim())} />
          </div>
          <div className="grid gap-1.5">
            <Label>Save edits as</Label>
            <div className="grid grid-cols-2 gap-2">
              {([["pr", "A pull request", "Nothing lands until it is reviewed."], ["commit", "Commits", `Straight onto ${branch || "the branch"}.`]] as const).map(([id, title, hint]) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setMode(id)}
                  className={`rounded-md border p-3 text-left text-sm ${mode === id ? "border-primary bg-accent" : ""}`}
                >
                  <span className="font-medium">{title}</span>
                  <span className="block text-xs text-muted-foreground">{hint}</span>
                </button>
              ))}
            </div>
          </div>
          {signedInToken ? (
            <p className="text-sm text-muted-foreground">Signed in with GitHub.</p>
          ) : (
            <>
              {authUrl && (
                <Button
                  variant="outline"
                  onClick={() => {
                    location.href = signInUrl(authUrl, location.origin + location.pathname);
                  }}
                >
                  Sign in with GitHub
                </Button>
              )}
              <div className="grid gap-1.5">
                <Label htmlFor="token">{authUrl ? "Or a fine-grained token" : "Fine-grained token"}</Label>
                <Input id="token" type="password" autoComplete="off" value={typed} onChange={(e) => setToken(e.target.value.trim())} />
                <p className="text-xs text-muted-foreground">
                  Needs <em>Contents</em> and <em>Pull requests</em>: read and write, on that one repository.
                </p>
              </div>
            </>
          )}
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
            Remember on this device
          </label>
          <Button disabled={!ok} onClick={() => onGithub({ owner, repo: name, branch, token, mode }, remember)}>Open</Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2"><HardDrive className="size-4" /> This machine</CardTitle>
          <CardDescription>
            Run <code>fid studio</code> in the product and open the address it prints. Local editing also gives you the
            dashboard, derive and the live preview.
          </CardDescription>
        </CardHeader>
      </Card>
    </div>
  );
}
