import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfirmDialog from "./ConfirmDialog";

type Remote = {
  name: string;
  url: string | null;
};

type RemotesPanelProps = {
  repoPath: string;
  pullStrategy: "merge" | "rebase" | "ff-only";
  fetchPrune: boolean;
  /** Called after an action changed the repository. */
  onChanged: () => void;
};

const BUTTON =
  "rounded px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]";
const INPUT =
  "min-w-0 rounded border border-neutral-300 bg-white px-2 py-1 text-xs dark:border-neutral-700 dark:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]";

export default function RemotesPanel({
  repoPath,
  pullStrategy,
  fetchPrune,
  onChanged,
}: RemotesPanelProps) {
  const [remotes, setRemotes] = useState<Remote[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [newUrl, setNewUrl] = useState("");
  const [editing, setEditing] = useState<{ name: string; url: string } | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  async function load() {
    try {
      const res = await invoke<Remote[]>("repo_list_remotes", { path: repoPath });
      setRemotes(Array.isArray(res) ? res : []);
      setLoadError(null);
    } catch {
      setRemotes([]);
      setLoadError("Could not read the remotes.");
    }
  }

  useEffect(() => {
    setEditing(null);
    setMessage(null);
    setError(null);
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoPath]);

  function failure(e: unknown, fallback: string) {
    setError(typeof e === "string" ? e : fallback);
    setMessage(null);
  }

  async function run(key: string, work: () => Promise<void>) {
    setBusy(key);
    setError(null);
    try {
      await work();
    } finally {
      setBusy(null);
    }
  }

  async function add() {
    if (!newName.trim() || !newUrl.trim()) {
      setError("Enter a name and a URL.");
      return;
    }
    await run("add", async () => {
      try {
        await invoke("repo_add_remote", {
          path: repoPath,
          name: newName.trim(),
          url: newUrl.trim(),
        });
        setNewName("");
        setNewUrl("");
        setMessage("Remote added.");
        await load();
      } catch (e) {
        failure(e, "Could not add that remote.");
      }
    });
  }

  async function saveUrl() {
    if (!editing) return;
    if (!editing.url.trim()) {
      setError("Enter a URL.");
      return;
    }
    await run(`save:${editing.name}`, async () => {
      try {
        await invoke("repo_set_remote_url", {
          path: repoPath,
          name: editing.name,
          url: editing.url.trim(),
        });
        setEditing(null);
        setMessage(`Updated ${editing.name}.`);
        await load();
      } catch (e) {
        failure(e, "Could not change that remote's URL.");
      }
    });
  }

  async function remove() {
    if (!removing) return;
    await run("remove", async () => {
      try {
        await invoke("repo_remove_remote", { path: repoPath, name: removing });
        setMessage(`Removed ${removing}.`);
        setRemoving(null);
        await load();
      } catch (e) {
        failure(e, "Could not remove that remote.");
      }
    });
  }

  async function fetch(name: string) {
    await run(`fetch:${name}`, async () => {
      try {
        await invoke("repo_fetch", {
          path: repoPath,
          name,
          prune: fetchPrune,
        });
        setMessage(`Fetched ${name}.`);
        onChanged();
      } catch (e) {
        failure(e, "Could not fetch from that remote.");
      }
    });
  }

  async function pull() {
    await run("pull", async () => {
      try {
        await invoke("repo_pull", { path: repoPath, strategy: pullStrategy });
        setMessage("Pulled the current branch.");
        onChanged();
      } catch (e) {
        failure(e, "Could not pull.");
      }
    });
  }

  return (
    <div className="text-sm">
      {loadError && (
        <p role="alert" className="mb-1 text-xs text-red-700 dark:text-red-400">
          {loadError}
        </p>
      )}

      {remotes.length === 0 && !loadError && (
        <p className="text-xs opacity-60">No remotes yet. Add one below.</p>
      )}

      <ul className="flex flex-col gap-0.5">
        {remotes.map((remote) => (
          <li key={remote.name} className="rounded px-1 py-0.5">
            {editing?.name === remote.name ? (
              <div className="flex items-center gap-1">
                <span className="shrink-0 font-medium">{remote.name}</span>
                <input
                  autoFocus
                  value={editing.url}
                  onChange={(event) =>
                    setEditing({ name: remote.name, url: event.target.value })
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Enter") saveUrl();
                    if (event.key === "Escape") setEditing(null);
                  }}
                  aria-label={`URL for ${remote.name}`}
                  spellCheck={false}
                  className={`${INPUT} flex-1 font-mono`}
                />
                <button
                  type="button"
                  onClick={saveUrl}
                  disabled={busy !== null}
                  className={`${BUTTON} shrink-0 font-medium`}
                >
                  {busy === `save:${remote.name}` ? "Saving…" : "Save"}
                </button>
                <button
                  type="button"
                  onClick={() => setEditing(null)}
                  disabled={busy !== null}
                  className={`${BUTTON} shrink-0`}
                >
                  Cancel
                </button>
              </div>
            ) : (
              <div className="flex items-center gap-1">
                <div className="min-w-0 flex-1">
                  <div className="truncate font-medium">{remote.name}</div>
                  <div
                    className="truncate font-mono text-xs opacity-60"
                    title={remote.url ?? undefined}
                  >
                    {remote.url ?? "no URL"}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => fetch(remote.name)}
                  disabled={busy !== null}
                  title={`Fetch ${remote.name}${fetchPrune ? " and prune deleted branches" : ""}`}
                  className={`${BUTTON} shrink-0`}
                >
                  {busy === `fetch:${remote.name}` ? "Fetching…" : "Fetch"}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setError(null);
                    setMessage(null);
                    setEditing({ name: remote.name, url: remote.url ?? "" });
                  }}
                  disabled={busy !== null}
                  title={`Change ${remote.name}'s URL`}
                  className={`${BUTTON} shrink-0`}
                >
                  Edit
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setError(null);
                    setMessage(null);
                    setRemoving(remote.name);
                  }}
                  disabled={busy !== null}
                  title={`Remove ${remote.name}`}
                  className={`${BUTTON} shrink-0 text-red-700 dark:text-red-400`}
                >
                  Remove
                </button>
              </div>
            )}
          </li>
        ))}
      </ul>

      <div className="mt-2 flex flex-col gap-1">
        <div className="flex items-center gap-1">
          <input
            value={newName}
            onChange={(event) => setNewName(event.target.value)}
            placeholder="name"
            aria-label="New remote name"
            spellCheck={false}
            className={`${INPUT} w-24 shrink-0`}
          />
          <input
            value={newUrl}
            onChange={(event) => setNewUrl(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") add();
            }}
            placeholder="https://host/owner/repo.git"
            aria-label="New remote URL"
            spellCheck={false}
            className={`${INPUT} flex-1 font-mono`}
          />
          <button
            type="button"
            onClick={add}
            disabled={busy !== null}
            className="shrink-0 rounded bg-[var(--accent)] px-2 py-1 text-xs text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Add remote
          </button>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={pull}
            disabled={busy !== null || remotes.length === 0}
            title={`Pull the current branch using ${pullStrategy}`}
            className={BUTTON}
          >
            {busy === "pull" ? "Pulling…" : `Pull (${pullStrategy})`}
          </button>
        </div>
      </div>

      {message && (
        <p role="status" className="mt-1 text-xs opacity-70">
          {message}
        </p>
      )}
      {error && (
        <p role="alert" className="mt-1 text-xs text-red-700 dark:text-red-400">
          {error}
        </p>
      )}

      {removing && (
        <ConfirmDialog
          title="Remove remote"
          description="The remote's address is deleted from this repository. Branches you already fetched are kept, so nothing is lost locally."
          target={removing}
          confirmLabel="Remove remote"
          busy={busy === "remove"}
          error={error}
          onConfirm={remove}
          onCancel={() => {
            setRemoving(null);
            setError(null);
          }}
        />
      )}
    </div>
  );
}
