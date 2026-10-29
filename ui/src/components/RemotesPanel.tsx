import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type RemotesPanelProps = {
  repoPath: string;
};

export default function RemotesPanel({ repoPath }: RemotesPanelProps) {
  const [remotes, setRemotes] = useState<string[]>([]);
  const [name, setName] = useState("origin");
  const [url, setUrl] = useState("");

  async function load() {
    try {
      const res: any = await invoke("repo_list_remotes", { path: repoPath });
      setRemotes(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  async function add() {
    if (!name || !url) return;
    await invoke("repo_add_remote", { path: repoPath, name, url });
    setUrl("");
    load();
  }

  async function fetch() {
    await invoke("repo_fetch", { path: repoPath, name: "origin" });
  }

  async function pull() {
    await invoke("repo_pull", { path: repoPath });
  }

  return (
    <div className="p-4">
      <h2 className="text-lg font-semibold mb-2">Remotes</h2>
      <div className="flex gap-2 mb-2">
        <input className="px-2 py-1 border rounded" value={name} onChange={(e) => setName(e.target.value)} />
        <input className="px-2 py-1 border rounded flex-1" value={url} onChange={(e) => setUrl(e.target.value)} placeholder="url" />
        <button className="px-3 py-1 bg-blue-600 text-white rounded" onClick={add}>Add</button>
      </div>
      <div className="flex gap-2 mb-2">
        <button className="px-3 py-1" onClick={fetch}>Fetch</button>
        <button className="px-3 py-1" onClick={pull}>Pull</button>
      </div>
      <ul className="space-y-1">
        {remotes.map((r) => (
          <li key={r}>{r}</li>
        ))}
      </ul>
    </div>
  );
}
