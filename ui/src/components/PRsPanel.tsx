import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type PRsPanelProps = {
  repoPath: string;
  onOpenAccounts?: () => void;
};

type PrInfo = {
  number: number;
  title: string;
  state: string;
};

export default function PRsPanel({ repoPath, onOpenAccounts }: PRsPanelProps) {
  const [prs, setPrs] = useState<PrInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setError(null);
    try {
      const res: any = await invoke("gh_list_repo_prs", { path: repoPath });
      setPrs(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not load the pull requests.");
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Pull Requests</div>
      {error ? (
        <div className="text-xs text-red-700 dark:text-red-400" role="alert">
          {error}
          {onOpenAccounts && (
            <button
              type="button"
              className="mt-1 block rounded px-2 py-1 text-xs bg-neutral-200 dark:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              onClick={onOpenAccounts}
            >
              Open Accounts
            </button>
          )}
        </div>
      ) : prs.length === 0 ? (
        <div className="text-xs opacity-60">No open pull requests</div>
      ) : (
        <ul className="space-y-1">
          {prs.map((p) => (
            <li key={p.number} className="text-xs truncate">
              <span className="opacity-60">#{p.number}</span> {p.title}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
