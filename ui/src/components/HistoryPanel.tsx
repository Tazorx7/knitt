import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type HistoryPanelProps = {
  repoPath: string;
  onSelectCommit?: (id: string) => void;
};

export default function HistoryPanel({ repoPath, onSelectCommit }: HistoryPanelProps) {
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
    <div className="panel p-4">
      <h2 className="text-lg font-semibold mb-2">History</h2>
      <ul className="space-y-2">
        {commits.map((c) => (
          <li key={c.id} className="border-b pb-2">
            <button
              type="button"
              onClick={() => onSelectCommit?.(c.id)}
              disabled={!onSelectCommit}
              aria-label={`View commit ${c.message.split("\n")[0]}`}
              className="w-full rounded px-1 py-1 text-left hover:bg-neutral-100 disabled:hover:bg-transparent dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              <div className="truncate font-medium">{c.message.split("\n")[0]}</div>
              <div className="truncate text-sm text-gray-500">{c.author}</div>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
