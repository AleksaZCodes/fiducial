import { useState } from "react";
import { GitPullRequest } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useSource } from "@/lib/source-context";

/**
 * Shown only when the source collects edits on a branch. Saves accumulate
 * there; this turns them into one pull request.
 */
export function ReviewBar({ version }: { version: number }) {
  const source = useSource();
  const [title, setTitle] = useState("Edit declarations");
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  if (!source.review) return null;
  // `version` changes when a save lands, so the branch name re-reads.
  const branch = (void version, source.review.branch());

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm" variant={branch ? "default" : "outline"} disabled={!branch}>
          <GitPullRequest /> {branch ? "Open pull request" : "No changes yet"}
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Open a pull request</DialogTitle>
          <DialogDescription>
            Everything saved so far is on <code>{branch}</code>. Nothing reaches the base branch until this is merged.
          </DialogDescription>
        </DialogHeader>
        <Input value={title} onChange={(e) => setTitle(e.target.value)} aria-label="Title" />
        <DialogFooter>
          <Button
            disabled={busy || !title.trim()}
            onClick={async () => {
              setBusy(true);
              try {
                const url = await source.review!.open(title.trim());
                setOpen(false);
                toast.success("Pull request opened", { action: { label: "View", onClick: () => window.open(url, "_blank") } });
              } catch (e) {
                toast.error((e as Error).message);
              } finally {
                setBusy(false);
              }
            }}
          >
            Open
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
