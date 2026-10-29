import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type StashPanelProps = {
  repoPath: string;
};

export default function StashPanel({ repoPath }: StashPanelProps) {
  const [stashes, setStashes] = useState<string[]>([]);

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
    await invoke("repo_stash_save", { path: repoPath, message: null });
    load();
  }

  async function pop() {
    await invoke("repo_stash_pop", { path: repoPath });
    load();
  }

  async function drop() {
    await invoke("repo_stash_drop", { path: repoPath });
    load();
  }

  return (
    <div className="p-4">
      <h2 className="text-lg font-semibold mb-2">Stash</h2>
      <div className="flex gap-2 mb-2">
        <button className="px-3 py-1 bg-yellow-600 text-white rounded" onClick={save}>Stash</button>
        <button className="px-3 py-1" onClick={pop}>Pop</button>
        <button className="px-3 py-1" onClick={drop}>Drop</button>
      </div>
      <ul className="space-y-1">
        {stashes.map((s, i) => (
          <li key={i}>{s}</li>
        ))}
      </ul>
    </div>
  );
}
