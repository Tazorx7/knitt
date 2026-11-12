import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfirmDialog from "./ConfirmDialog";

type StashPanelProps = {
  repoPath: string;
  confirmDrop: boolean;
};

export default function StashPanel({ repoPath, confirmDrop }: StashPanelProps) {
  const [stashes, setStashes] = useState<string[]>([]);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function load() {
    try {
      const res: any = await invoke("repo_stash_list", { path: repoPath });
      setStashes(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  async function save() {
    setError(null);
    try {
      await invoke("repo_stash_save", { path: repoPath, message: null });
      load();
    } catch {
      setError("Could not stash your changes.");
    }
  }

  async function pop() {
    setError(null);
    try {
      await invoke("repo_stash_pop", { path: repoPath });
      load();
    } catch {
      setError("Could not pop the stash.");
    }
  }

  async function drop() {
    setBusy(true);
    setError(null);
    try {
      await invoke("repo_stash_drop", { path: repoPath });
      setConfirming(false);
      load();
    } catch {
      setError("Could not drop the stash.");
    } finally {
      setBusy(false);
    }
  }

  function requestDrop() {
    if (confirmDrop) {
      setConfirming(true);
    } else {
      drop();
    }
  }

  return (
    <div className="panel p-4">
      <h2 className="text-lg font-semibold mb-2">Stash</h2>
      <div className="flex gap-2 mb-2">
        <button className="px-3 py-1 bg-yellow-600 text-white rounded" onClick={save}>Stash</button>
        <button className="px-3 py-1" onClick={pop}>Pop</button>
        <button className="px-3 py-1" onClick={requestDrop}>Drop</button>
      </div>
      <ul className="space-y-1">
        {stashes.map((s, i) => (
          <li key={i}>{s}</li>
        ))}
      </ul>
      {error && (
        <p role="status" className="mt-2 text-xs text-red-700 dark:text-red-400">
          {error}
        </p>
      )}

      {confirming && (
        <ConfirmDialog
          title="Drop stash"
          description="Permanently removes the latest stash entry. The changes it holds cannot be recovered from Knitt afterwards."
          target={stashes[0]}
          confirmLabel="Drop stash"
          busy={busy}
          error={error}
          onConfirm={drop}
          onCancel={() => setConfirming(false)}
        />
      )}
    </div>
  );
}
