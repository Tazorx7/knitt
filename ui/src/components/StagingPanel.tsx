import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type StagingPanelProps = {
  repoPath: string;
  files: string[];
  onRefresh?: () => void;
};

export default function StagingPanel({ repoPath, files, onRefresh }: StagingPanelProps) {
  const [selected, setSelected] = useState<string[]>([]);

  async function stageSelected() {
    if (selected.length === 0) return;
    await invoke("repo_stage_files", { path: repoPath, files: selected });
    setSelected([]);
    onRefresh?.();
  }

  async function stageAll() {
    await invoke("repo_stage_all", { path: repoPath });
    onRefresh?.();
  }

  return (
    <div className="p-4">
      <h2 className="text-lg font-semibold mb-2">Changes</h2>
      <div className="flex gap-2 mb-2">
        <button className="px-3 py-1 bg-green-600 text-white rounded" onClick={stageAll}>
          Stage All
        </button>
        <button className="px-3 py-1 bg-blue-600 text-white rounded" onClick={stageSelected}>
          Stage Selected
        </button>
      </div>
      <ul className="space-y-1">
        {files.map((f) => (
          <li key={f} className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={selected.includes(f)}
              onChange={(e) => {
                if (e.target.checked) setSelected([...selected, f]);
                else setSelected(selected.filter((x) => x !== f));
              }}
            />
            <span>{f}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
