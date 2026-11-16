import { useState } from "react";
import { GitBranch, HardDrive } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { GithubTarget } from "@/lib/sources/github";

/**
 * The first screen when Studio was not opened by `fid studio`.
 *
 * GitHub sign-in is a fine-grained token today. A browser cannot finish
 * GitHub's OAuth web flow on its own (the code exchange needs a client secret),
 * so OAuth is a small Worker that holds the secret; until one is configured
 * the token is the honest, working path, and it never leaves this browser.
 */
export function Connect({ onGithub }: { onGithub: (t: GithubTarget, remember: boolean) => void }) {
  const [repo, setRepo] = useState("");
  const [branch, setBranch] = useState("main");
  const [token, setToken] = useState("");
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
          <CardDescription>Works from any browser, including a phone. Saves are commits.</CardDescription>
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
            <Label htmlFor="token">Fine-grained token</Label>
            <Input id="token" type="password" autoComplete="off" value={token} onChange={(e) => setToken(e.target.value.trim())} />
            <p className="text-xs text-muted-foreground">
              Needs <em>Contents: read and write</em> on that one repository. It is sent only to api.github.com.
            </p>
          </div>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
            Remember on this device
          </label>
          <Button disabled={!ok} onClick={() => onGithub({ owner, repo: name, branch, token }, remember)}>Open</Button>
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
