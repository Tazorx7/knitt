import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

type RepoInfo = {
  path: string;
  name: string;
  branch?: string;
};

type CloneDialogProps = {
  onCloned: (info: RepoInfo) => void;
  onClose: () => void;
};

type CloneProgress = {
  received: number;
  total: number;
};

/** Last path segment of an HTTPS or SSH URL, without a trailing `.git`. */
function repoNameFromUrl(value: string): string {
  const trimmed = value.trim().replace(/\/+$/, "").replace(/\.git$/i, "");
  const parts = trimmed.split(/[/:]/).filter(Boolean);
  return parts[parts.length - 1] ?? "";
}

export default function CloneDialog({ onCloned, onClose }: CloneDialogProps) {
  const [url, setUrl] = useState("");
  const [parent, setParent] = useState("");
  const [name, setName] = useState("");
  const [nameEdited, setNameEdited] = useState(false);
  const [branch, setBranch] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<CloneProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const urlRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    urlRef.current?.focus();
  }, []);

  // Progress arrives from the backend while the blocking clone runs.
  useEffect(() => {
    const pending = listen<CloneProgress>("clone-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      pending.then((unlisten) => unlisten());
    };
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

  function updateUrl(value: string) {
    setUrl(value);
    if (!nameEdited) {
      setName(repoNameFromUrl(value));
    }
  }

  async function browse() {
    try {
      const selected = await openDialog({ directory: true, multiple: false });
      if (typeof selected === "string") {
        setParent(selected);
      }
    } catch {
      setError("Could not open the folder picker.");
    }
  }

  async function clone() {
    if (busy) return;
    setError(null);
    if (!url.trim()) {
      setError("Enter a repository URL.");
      return;
    }
    if (!parent.trim()) {
      setError("Choose a parent folder.");
      return;
    }
    const folder = name.trim() || repoNameFromUrl(url);
    if (!folder) {
      setError("Enter a folder name.");
      return;
    }
    const destination = `${parent.trim().replace(/\/+$/, "")}/${folder}`;
    setBusy(true);
    setProgress(null);
    try {
      const info = await invoke<RepoInfo>("repo_clone", {
        url: url.trim(),
        destination,
        branch: branch.trim() || null,
      });
      onCloned(info);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not clone the repository.");
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }

  const percent =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.received / progress.total) * 100))
      : null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="clone-title"
        className="w-full max-w-md rounded-lg border border-neutral-200 bg-white p-4 shadow-xl dark:border-neutral-700 dark:bg-neutral-900"
      >
        <h2 id="clone-title" className="text-sm font-semibold">
          Clone repository
        </h2>
        <p className="mt-1 text-xs opacity-70">
          HTTPS uses your signed-in GitHub account; SSH uses your agent key.
        </p>

        <label className="mt-4 block text-xs">
          <span className="block pb-1 opacity-70">Repository URL</span>
          <input
            ref={urlRef}
            type="text"
            value={url}
            onChange={(event) => updateUrl(event.target.value)}
            placeholder="https://github.com/owner/repository.git"
            disabled={busy}
            className="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm disabled:opacity-60 dark:border-neutral-700 dark:bg-neutral-900"
          />
        </label>

        <label className="mt-3 block text-xs">
          <span className="block pb-1 opacity-70">Parent folder</span>
          <div className="flex gap-2">
            <input
              type="text"
              value={parent}
              onChange={(event) => setParent(event.target.value)}
              placeholder="/home/you/projects"
              disabled={busy}
              className="min-w-0 flex-1 rounded border border-neutral-300 bg-white px-2 py-1 text-sm disabled:opacity-60 dark:border-neutral-700 dark:bg-neutral-900"
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
          <span className="block pb-1 opacity-70">Folder name</span>
          <input
            type="text"
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              setNameEdited(true);
            }}
            placeholder="repository"
            disabled={busy}
            className="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm disabled:opacity-60 dark:border-neutral-700 dark:bg-neutral-900"
          />
        </label>

        <label className="mt-3 block text-xs">
          <span className="block pb-1 opacity-70">Branch</span>
          <input
            type="text"
            value={branch}
            onChange={(event) => setBranch(event.target.value)}
            placeholder="Leave blank for the default branch"
            disabled={busy}
            className="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm disabled:opacity-60 dark:border-neutral-700 dark:bg-neutral-900"
          />
        </label>

        {busy && (
          <div className="mt-3 text-xs" role="status">
            <div className="flex items-center justify-between pb-1 opacity-70">
              <span>Cloning…</span>
              {percent !== null && <span>{percent}%</span>}
            </div>
            <div className="h-1 overflow-hidden rounded bg-neutral-200 dark:bg-neutral-800">
              <div
                className="h-full bg-[var(--accent)] transition-[width]"
                style={{ width: `${percent ?? 5}%` }}
              />
            </div>
          </div>
        )}

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
            onClick={clone}
            disabled={busy}
            className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            {busy ? "Cloning…" : "Clone"}
          </button>
        </div>
      </div>
    </div>
  );
}
