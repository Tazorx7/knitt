import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type PRsPanelProps = {
  owner?: string;
  repo?: string;
};

export default function PRsPanel({ owner, repo }: PRsPanelProps) {
  const [prs, setPrs] = useState<any[]>([]);

  async function load() {
    if (!owner || !repo) return;
    try {
      const res: any = await invoke("gh_list_prs", { owner, repo });
      setPrs(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [owner, repo]);

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Pull Requests</div>
      <ul className="space-y-1">
        {prs.map((p) => (
          <li key={p.number} className="text-xs truncate">
            #{p.number} {p.title}
          </li>
        ))}
      </ul>
    </div>
  );
}
