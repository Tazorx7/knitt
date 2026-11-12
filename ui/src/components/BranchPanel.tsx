import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type BranchPanelProps = {
  repoPath: string;
  currentBranch?: string;
  onCheckedOut: () => void;
};

export default function BranchPanel({
  repoPath,
  currentBranch,
  onCheckedOut,
}: BranchPanelProps) {
  const [branches, setBranches] = useState<string[]>([]);
  const [newBranch, setNewBranch] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      const res: any = await invoke("repo_branches", { path: repoPath });
      setBranches(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
      setError("Could not load branches.");
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  async function create() {
    const name = newBranch.trim();
    if (!name) return;
    setBusy("create");
    setError(null);
    try {
      await invoke("repo_create_branch", { path: repoPath, name });
      setNewBranch("");
      await load();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not create that branch.");
    } finally {
      setBusy(null);
    }
  }

  async function checkout(name: string) {
    if (name === currentBranch || busy) return;
    setBusy(name);
    setError(null);
    try {
      await invoke("repo_checkout_branch", { path: repoPath, name });
      await load();
      onCheckedOut();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not switch branches.");
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="mb-2">
      <div className="mb-2 flex gap-1">
        <input
          className="min-w-0 flex-1 rounded border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] dark:border-neutral-700 dark:bg-neutral-900"
          value={newBranch}
          onChange={(e) => setNewBranch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") create();
          }}
          placeholder="New branch"
          aria-label="New branch name"
        />
        <button
          type="button"
          onClick={create}
          disabled={!newBranch.trim() || busy !== null}
          className="rounded bg-neutral-900 px-2 py-1 text-xs text-white hover:bg-neutral-800 disabled:opacity-50 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
        >
          Create
        </button>
      </div>

      {error && (
        <p role="alert" className="mb-2 text-xs text-red-700 dark:text-red-400">
          {error}
        </p>
      )}

      <ul className="flex flex-col gap-0.5">
        {branches.map((branch) => {
          const isCurrent = branch === currentBranch;
          return (
            <li key={branch}>
              <button
                type="button"
                onClick={() => checkout(branch)}
                disabled={isCurrent || busy !== null}
                aria-current={isCurrent ? "true" : undefined}
                title={isCurrent ? "Current branch" : `Check out ${branch}`}
                className={`flex w-full items-center justify-between rounded px-2 py-1 text-left text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
                  isCurrent
                    ? "bg-neutral-100 font-medium dark:bg-neutral-900"
                    : "hover:bg-neutral-100 dark:hover:bg-neutral-900"
                } ${busy !== null && !isCurrent ? "opacity-60" : ""}`}
              >
                <span className="truncate">{branch}</span>
                <span className="ml-2 shrink-0 text-xs opacity-60">
                  {isCurrent ? "current" : busy === branch ? "switching…" : ""}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
