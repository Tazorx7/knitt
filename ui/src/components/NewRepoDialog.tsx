import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

type RepoInfo = {
  path: string;
  name: string;
  branch?: string;
};

type NewRepoDialogProps = {
  defaultBranch: string;
  onCreated: (info: RepoInfo) => void;
  onClose: () => void;
};

/**
 * Creates a new repository in an existing or new folder. The initial branch
 * starts from the configured default branch but stays editable, and nothing is
 * written to the user's global Git config.
 */
export default function NewRepoDialog({
  defaultBranch,
  onCreated,
  onClose,
}: NewRepoDialogProps) {
  const [folder, setFolder] = useState("");
  const [branch, setBranch] = useState(defaultBranch);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const folderRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    folderRef.current?.focus();
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !busy) {
        event.preventDefault();
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [busy, onClose]);

  async function browse() {
    try {
      const selected = await openDialog({ directory: true, multiple: false });
      if (typeof selected === "string") {
        setFolder(selected);
      }
    } catch {
      setError("Could not open the folder picker.");
    }
  }

  async function create() {
    if (busy) return;
    const trimmedFolder = folder.trim();
    if (!trimmedFolder) {
      setError("Choose a folder for the new repository.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const info = await invoke<RepoInfo>("repo_init", {
        path: trimmedFolder,
        branch: branch.trim(),
      });
      onCreated(info);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not create the repository.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-repo-title"
        className="w-full max-w-md rounded-lg border border-neutral-200 bg-white p-4 shadow-xl dark:border-neutral-700 dark:bg-neutral-900"
      >
        <h2 id="new-repo-title" className="text-sm font-semibold">
          New repository
        </h2>
        <p className="mt-1 text-xs opacity-70">
          Creates a repository in the folder you choose. Existing files are
          left untouched.
        </p>

        <label className="mt-4 block text-xs">
          <span className="block pb-1 opacity-70">Folder</span>
          <div className="flex gap-2">
            <input
              ref={folderRef}
              type="text"
              value={folder}
              onChange={(event) => setFolder(event.target.value)}
              placeholder="/home/you/projects/new-project"
              className="min-w-0 flex-1 rounded border border-neutral-300 bg-white px-2 py-1 text-sm dark:border-neutral-700 dark:bg-neutral-900"
            />
            <button
              type="button"
              onClick={browse}
              disabled={busy}
              className="shrink-0 rounded px-2 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Browse…
            </button>
          </div>
        </label>

        <label className="mt-3 block text-xs">
          <span className="block pb-1 opacity-70">Initial branch</span>
          <input
            type="text"
            value={branch}
            onChange={(event) => setBranch(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                create();
              }
            }}
            className="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm dark:border-neutral-700 dark:bg-neutral-900"
          />
        </label>

        {error && (
          <p role="status" className="mt-2 text-xs text-red-700 dark:text-red-400">
            {error}
          </p>
        )}

        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="rounded px-3 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={create}
            disabled={busy}
            className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            {busy ? "Creating…" : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}
