import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type BranchPanelProps = {
  repoPath: string;
};

export default function BranchPanel({ repoPath }: BranchPanelProps) {
  const [branches, setBranches] = useState<string[]>([]);
  const [newBranch, setNewBranch] = useState("");

  async function load() {
    try {
      const res: any = await invoke("repo_branches", { path: repoPath });
      setBranches(Array.isArray(res) ? res : []);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  async function create() {
    if (!newBranch) return;
    await invoke("repo_create_branch", { path: repoPath, name: newBranch });
    setNewBranch("");
    load();
  }

  async function checkout(name: string) {
    await invoke("repo_checkout_branch", { path: repoPath, name });
    load();
  }

  return (
    <div className="p-4">
      <h2 className="text-lg font-semibold mb-2">Branches</h2>
      <div className="flex gap-2 mb-2">
        <input
          className="px-2 py-1 border rounded"
          value={newBranch}
          onChange={(e) => setNewBranch(e.target.value)}
          placeholder="new branch"
        />
        <button className="px-3 py-1 bg-blue-600 text-white rounded" onClick={create}>
          Create
        </button>
      </div>
      <ul className="space-y-1">
        {branches.map((b) => (
          <li key={b} className="flex justify-between items-center">
            <span>{b}</span>
            <button className="px-2 py-1 text-sm" onClick={() => checkout(b)}>
              Checkout
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
