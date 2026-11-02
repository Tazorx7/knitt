import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type ActionsPanelProps = {
  owner?: string;
  repo?: string;
};

export default function ActionsPanel({ owner, repo }: ActionsPanelProps) {
  const [runs, setRuns] = useState<any[]>([]);

  async function load() {
    if (!owner || !repo) return;
    try {
      const res: any = await invoke("gh_list_workflow_runs", { owner, repo });
      setRuns(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [owner, repo]);

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Actions</div>
      <ul className="space-y-1">
        {runs.map((r) => (
          <li key={r.id} className="text-xs truncate">
            {r.name} - {r.status} {r.conclusion || ""}
          </li>
        ))}
      </ul>
    </div>
  );
}
