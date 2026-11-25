import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type IssuesPanelProps = {
  repoPath: string;
  onOpenAccounts?: () => void;
};

type IssueInfo = {
  number: number;
  title: string;
  state: string;
};

export default function IssuesPanel({ repoPath, onOpenAccounts }: IssuesPanelProps) {
  const [issues, setIssues] = useState<IssueInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setError(null);
    try {
      const res: any = await invoke("gh_list_repo_issues", { path: repoPath });
      setIssues(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not load the issues.");
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Issues</div>
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
      ) : issues.length === 0 ? (
        <div className="text-xs opacity-60">No open issues</div>
      ) : (
        <ul className="space-y-1">
          {issues.map((i) => (
            <li key={i.number} className="text-xs truncate">
              <span className="opacity-60">#{i.number}</span> {i.title}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
