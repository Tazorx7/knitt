import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfirmDialog from "./ConfirmDialog";

type BranchPanelProps = {
  repoPath: string;
  currentBranch?: string;
  onCheckedOut: () => void;
};

type BranchOverview = {
  name: string;
  upstream: string | null;
};

export default function BranchPanel({
  repoPath,
  currentBranch,
  onCheckedOut,
}: BranchPanelProps) {
  const [branches, setBranches] = useState<BranchOverview[]>([]);
  const [newBranch, setNewBranch] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [menuFor, setMenuFor] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<{ from: string; value: string } | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [merging, setMerging] = useState<string | null>(null);
  const [rebasing, setRebasing] = useState<string | null>(null);
  const [integrateBusy, setIntegrateBusy] = useState(false);
  const [integrateError, setIntegrateError] = useState<string | null>(null);

  async function load() {
    try {
      const res = await invoke<BranchOverview[]>("repo_branch_overview", { path: repoPath });
      setBranches(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
      setError("Could not load branches.");
    }
  }

  useEffect(() => {
    load();
    setMenuFor(null);
    setRenaming(null);
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
    setMenuFor(null);
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

  async function rename() {
    if (!renaming) return;
    const next = renaming.value.trim();
    if (!next || next === renaming.from) {
      setRenaming(null);
      return;
    }
    setBusy("rename");
    setError(null);
    try {
      await invoke("repo_rename_branch", {
        path: repoPath,
        oldName: renaming.from,
        newName: next,
      });
      setRenaming(null);
      await load();
      if (renaming.from === currentBranch) {
        onCheckedOut();
      }
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not rename that branch.");
    } finally {
      setBusy(null);
    }
  }

  async function publish(name: string) {
    setBusy(`publish:${name}`);
    setError(null);
    try {
      await invoke("repo_publish_branch", { path: repoPath, branch: name });
      await load();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not publish that branch.");
    } finally {
      setBusy(null);
    }
  }

  /** Merges the chosen branch into the checked-out branch. */
  async function mergeBranch() {
    if (!merging) return;
    setIntegrateBusy(true);
    setIntegrateError(null);
    try {
      await invoke("repo_merge", { path: repoPath, branch: merging });
      setMerging(null);
      await load();
      onCheckedOut();
    } catch (e) {
      setIntegrateError(typeof e === "string" ? e : "Could not merge that branch.");
    } finally {
      setIntegrateBusy(false);
    }
  }

  /** Replays the checked-out branch on top of the chosen branch. */
  async function rebaseBranch() {
    if (!rebasing) return;
    setIntegrateBusy(true);
    setIntegrateError(null);
    try {
      await invoke("repo_rebase", { path: repoPath, branch: rebasing });
      setRebasing(null);
      await load();
      onCheckedOut();
    } catch (e) {
      setIntegrateError(typeof e === "string" ? e : "Could not rebase onto that branch.");
    } finally {
      setIntegrateBusy(false);
    }
  }

  async function removeBranch() {
    if (!deleting) return;
    setDeleteBusy(true);
    setDeleteError(null);
    try {
      await invoke("repo_delete_branch", { path: repoPath, name: deleting });
      setDeleting(null);
      await load();
    } catch (e) {
      setDeleteError(typeof e === "string" ? e : "Could not delete that branch.");
    } finally {
      setDeleteBusy(false);
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
          const isCurrent = branch.name === currentBranch;
          return (
            <li key={branch.name}>
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => checkout(branch.name)}
                  disabled={isCurrent || busy !== null}
                  aria-current={isCurrent ? "true" : undefined}
                  title={isCurrent ? "Current branch" : `Check out ${branch.name}`}
                  className={`flex min-w-0 flex-1 items-center justify-between rounded px-2 py-1 text-left text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
                    isCurrent
                      ? "bg-neutral-100 font-medium dark:bg-neutral-900"
                      : "hover:bg-neutral-100 dark:hover:bg-neutral-900"
                  } ${busy !== null && !isCurrent ? "opacity-60" : ""}`}
                >
                  <span className="truncate">{branch.name}</span>
                  <span className="ml-2 shrink-0 text-xs opacity-60">
                    {isCurrent
                      ? "current"
                      : branch.upstream
                        ? branch.upstream.replace(/^[^/]+\//, "")
                        : "not published"}
                  </span>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setMenuFor(menuFor === branch.name ? null : branch.name);
                    setRenaming(null);
                  }}
                  aria-label={`Actions for ${branch.name}`}
                  aria-expanded={menuFor === branch.name}
                  className="shrink-0 rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                >
                  ⋯
                </button>
              </div>

              {menuFor === branch.name && (
                <div className="ml-2 flex flex-wrap items-center gap-1 pb-1 pt-0.5">
                  <button
                    type="button"
                    onClick={() => {
                      setRenaming({ from: branch.name, value: branch.name });
                      setMenuFor(null);
                    }}
                    className="rounded px-2 py-0.5 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                  >
                    Rename
                  </button>
                  {!branch.upstream && (
                    <button
                      type="button"
                      onClick={() => {
                        setMenuFor(null);
                        publish(branch.name);
                      }}
                      disabled={busy !== null}
                      className="rounded px-2 py-0.5 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                    >
                      {busy === `publish:${branch.name}` ? "Publishing…" : "Publish"}
                    </button>
                  )}
                  {!isCurrent && (
                    <>
                      <button
                        type="button"
                        onClick={() => {
                          setIntegrateError(null);
                          setMerging(branch.name);
                          setMenuFor(null);
                        }}
                        disabled={busy !== null}
                        title={
                          currentBranch
                            ? `Merge ${branch.name} into ${currentBranch}`
                            : undefined
                        }
                        className="rounded px-2 py-0.5 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                      >
                        Merge into current
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          setIntegrateError(null);
                          setRebasing(branch.name);
                          setMenuFor(null);
                        }}
                        disabled={busy !== null}
                        title={
                          currentBranch
                            ? `Replay ${currentBranch} on top of ${branch.name}`
                            : undefined
                        }
                        className="rounded px-2 py-0.5 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                      >
                        Rebase onto
                      </button>
                    </>
                  )}
                  <button
                    type="button"
                    onClick={() => {
                      setDeleteError(null);
                      setDeleting(branch.name);
                      setMenuFor(null);
                    }}
                    disabled={isCurrent}
                    title={isCurrent ? "Check out another branch first" : undefined}
                    className="rounded px-2 py-0.5 text-xs text-red-700 hover:bg-red-50 disabled:opacity-40 dark:text-red-400 dark:hover:bg-red-950 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                  >
                    Delete
                  </button>
                </div>
              )}

              {renaming?.from === branch.name && (
                <div className="ml-2 flex items-center gap-1 pb-1 pt-0.5">
                  <input
                    autoFocus
                    value={renaming.value}
                    onChange={(e) => setRenaming({ ...renaming, value: e.target.value })}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") rename();
                      if (e.key === "Escape") setRenaming(null);
                    }}
                    aria-label={`New name for ${branch.name}`}
                    className="min-w-0 flex-1 rounded border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] dark:border-neutral-700 dark:bg-neutral-900"
                  />
                  <button
                    type="button"
                    onClick={rename}
                    disabled={busy === "rename" || !renaming.value.trim()}
                    className="rounded bg-neutral-900 px-2 py-1 text-xs text-white hover:bg-neutral-800 disabled:opacity-50 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                  >
                    Save
                  </button>
                  <button
                    type="button"
                    onClick={() => setRenaming(null)}
                    className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                  >
                    Cancel
                  </button>
                </div>
              )}
            </li>
          );
        })}
      </ul>

      {deleting && (
        <ConfirmDialog
          title="Delete branch"
          description="The branch is removed from your repository. Branches with unmerged commits are refused, so no commits are lost."
          target={deleting}
          confirmLabel="Delete branch"
          busy={deleteBusy}
          error={deleteError}
          onConfirm={removeBranch}
          onCancel={() => {
            setDeleting(null);
            setDeleteError(null);
          }}
        />
      )}

      {merging && (
        <ConfirmDialog
          title="Merge branch"
          description={`Merges ${merging} into the current branch. If the histories conflict, the merge stays in progress so you can resolve the files.`}
          target={currentBranch ?? undefined}
          confirmLabel="Merge"
          busy={integrateBusy}
          error={integrateError}
          onConfirm={mergeBranch}
          onCancel={() => {
            setMerging(null);
            setIntegrateError(null);
          }}
        />
      )}

      {rebasing && (
        <ConfirmDialog
          title="Rebase onto branch"
          description={`Replays the commits on the current branch on top of ${rebasing}. The commits get new ids; conflicts stop the rebase so you can resolve them or abort.`}
          target={currentBranch ?? undefined}
          confirmLabel="Rebase"
          busy={integrateBusy}
          error={integrateError}
          onConfirm={rebaseBranch}
          onCancel={() => {
            setRebasing(null);
            setIntegrateError(null);
          }}
        />
      )}
    </div>
  );
}
