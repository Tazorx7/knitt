import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type TagsPanelProps = {
  repoPath: string;
};

export default function TagsPanel({ repoPath }: TagsPanelProps) {
  const [tags, setTags] = useState<string[]>([]);
  const [name, setName] = useState("");

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
    await invoke("repo_create_tag", { path: repoPath, name });
    setName("");
    load();
  }

  async function push(t: string) {
    await invoke("repo_push_tag", { path: repoPath, name: t });
  }

  async function del(t: string) {
    await invoke("repo_delete_tag", { path: repoPath, name: t });
    load();
  }

  return (
    <div className="p-4">
      <h2 className="text-lg font-semibold mb-2">Tags</h2>
      <div className="flex gap-2 mb-2">
        <input className="px-2 py-1 border rounded" value={name} onChange={(e) => setName(e.target.value)} placeholder="tag name" />
        <button className="px-3 py-1 bg-blue-600 text-white rounded" onClick={create}>Create</button>
      </div>
      <ul className="space-y-1">
        {tags.map((t) => (
          <li key={t} className="flex justify-between items-center">
            <span>{t}</span>
            <div className="flex gap-2">
              <button className="px-2 py-1 text-sm" onClick={() => push(t)}>Push</button>
              <button className="px-2 py-1 text-sm" onClick={() => del(t)}>Delete</button>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
