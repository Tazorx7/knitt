import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type HistoryPanelProps = {
  repoPath: string;
};

export default function HistoryPanel({ repoPath }: HistoryPanelProps) {
  const [commits, setCommits] = useState<any[]>([]);

  async function load() {
    try {
      const res: any = await invoke("repo_log", { path: repoPath, limit: 50 });
      setCommits(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  return (
    <div className="p-4">
      <h2 className="text-lg font-semibold mb-2">History</h2>
      <ul className="space-y-2">
        {commits.map((c) => (
          <li key={c.id} className="border-b pb-2">
            <div className="font-medium">{c.message}</div>
            <div className="text-sm text-gray-500">{c.author}</div>
          </li>
        ))}
      </ul>
    </div>
  );
}
