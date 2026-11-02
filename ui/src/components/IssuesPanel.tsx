import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type IssuesPanelProps = {
  repoPath: string;
};

type IssueInfo = {
  number: number;
  title: string;
  state: string;
};

export default function IssuesPanel({ repoPath }: IssuesPanelProps) {
  const [issues, setIssues] = useState<IssueInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setError(null);
    try {
      const res: any = await invoke("gh_list_repo_issues", { path: repoPath });
      setIssues(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Issues</div>
      {error ? (
        <div className="text-xs opacity-60">Unable to load issues</div>
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
