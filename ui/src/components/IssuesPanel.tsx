import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type IssuesPanelProps = {
  owner?: string;
  repo?: string;
};

export default function IssuesPanel({ owner, repo }: IssuesPanelProps) {
  const [issues, setIssues] = useState<any[]>([]);

  async function load() {
    if (!owner || !repo) return;
    try {
      const res: any = await invoke("gh_list_issues", { owner, repo });
      setIssues(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [owner, repo]);

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Issues</div>
      <ul className="space-y-1">
        {issues.map((i) => (
          <li key={i.number} className="text-xs truncate">
            #{i.number} {i.title}
          </li>
        ))}
      </ul>
    </div>
  );
}
