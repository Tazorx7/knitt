import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfirmDialog from "./ConfirmDialog";

type TagsPanelProps = {
  repoPath: string;
  confirmDelete: boolean;
};

export default function TagsPanel({ repoPath, confirmDelete }: TagsPanelProps) {
  const [tags, setTags] = useState<string[]>([]);
  const [name, setName] = useState("");
  const [pendingTag, setPendingTag] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function load() {
    try {
      const res: any = await invoke("repo_list_tags", { path: repoPath });
      setTags(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  async function create() {
    if (!name) return;
    setError(null);
    try {
      await invoke("repo_create_tag", { path: repoPath, name });
      setName("");
      load();
    } catch {
      setError("Could not create that tag.");
    }
  }

  async function push(t: string) {
    setError(null);
    try {
      await invoke("repo_push_tag", { path: repoPath, name: t });
    } catch {
      setError("Could not push that tag.");
    }
  }

  async function removeTag(t: string) {
    setBusy(true);
    setError(null);
    try {
      await invoke("repo_delete_tag", { path: repoPath, name: t });
      setPendingTag(null);
      load();
    } catch {
      setError("Could not delete that tag.");
    } finally {
      setBusy(false);
    }
  }

  function requestDelete(t: string) {
    if (confirmDelete) {
      setPendingTag(t);
    } else {
      removeTag(t);
    }
  }

  return (
    <div className="panel p-4">
      <h2 className="text-lg font-semibold mb-2">Tags</h2>
      <div className="flex gap-2 mb-2">
        <input className="px-2 py-1 border rounded" value={name} onChange={(e) => setName(e.target.value)} placeholder="tag name" />
        <button className="px-3 py-1 bg-[var(--accent)] text-white rounded" onClick={create}>Create</button>
      </div>
      <ul className="space-y-1">
        {tags.map((t) => (
          <li key={t} className="flex justify-between items-center">
            <span>{t}</span>
            <div className="flex gap-2">
              <button className="px-2 py-1 text-sm" onClick={() => push(t)}>Push</button>
              <button className="px-2 py-1 text-sm" onClick={() => requestDelete(t)}>Delete</button>
            </div>
          </li>
        ))}
      </ul>
      {error && (
        <p role="status" className="mt-2 text-xs text-red-700 dark:text-red-400">
          {error}
        </p>
      )}

      {pendingTag && (
        <ConfirmDialog
          title="Delete tag"
          description="Deletes this tag from the local repository. A tag already pushed to the remote is not removed there."
          target={pendingTag}
          confirmLabel="Delete tag"
          busy={busy}
          error={error}
          onConfirm={() => removeTag(pendingTag)}
          onCancel={() => setPendingTag(null)}
        />
      )}
    </div>
  );
}
