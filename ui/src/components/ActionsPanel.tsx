import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type ActionsPanelProps = {
  repoPath: string;
};

type WorkflowRun = {
  id: number;
  name: string;
  status: string;
  conclusion: string | null;
};

export default function ActionsPanel({ repoPath }: ActionsPanelProps) {
  const [runs, setRuns] = useState<WorkflowRun[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setError(null);
    try {
      const res: any = await invoke("gh_list_repo_workflow_runs", { path: repoPath });
      setRuns(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Actions</div>
      {error ? (
        <div className="text-xs opacity-60">Unable to load workflow runs</div>
      ) : runs.length === 0 ? (
        <div className="text-xs opacity-60">No recent workflow runs</div>
      ) : (
        <ul className="space-y-1">
          {runs.map((r) => (
            <li key={r.id} className="text-xs truncate">
              <span className="opacity-60">{r.conclusion ?? r.status}</span> {r.name}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
