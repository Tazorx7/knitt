import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";


import { open as openDialog } from "@tauri-apps/plugin-dialog";
import Header from "./components/Header";

type Account = {
  id: string;
  host: string;
  login: string;
  name?: string;
  avatar_url: string;
};

type FileStatus = {
  path: string;
  status: string;
  staged: boolean;
  unstaged: boolean;
};

function App() {
  
  const [accounts, setAccounts] = useState<Account[]>([]);
  
  const [currentRepo, setCurrentRepo] = useState<string | null>(null);
  const [files, setFiles] = useState<FileStatus[]>([]);
  
  
  const [activeTab, setActiveTab] = useState("changes");

  useEffect(() => {
    
    invoke<Account[]>("auth_list_accounts").then(setAccounts).catch(() => setAccounts([]));
  }, []);

  async function startAuth() {
    await invoke("auth_start_device_flow", { host: "github.com" });
    
  }

  async function openRepo() {
    const selected = await openDialog({ directory: true });
    if (!selected || Array.isArray(selected)) return;
    try {
      await invoke("repo_open", { path: selected });
      setCurrentRepo(selected as string);
      refreshStatus(selected as string);
    } catch (e) {
      console.error(e);
    }
  }

  async function refreshStatus(path: string) {
    const res: any = await invoke("repo_status", { path });
    setFiles(res.files || []);
  }

  async function commit() {
    if (!currentRepo) return;
    const message = prompt("Commit message");
    if (!message) return;
    await invoke("repo_commit", { path: currentRepo, message });
    refreshStatus(currentRepo);
  }

  async function push() {
    if (!currentRepo) return;
    await invoke("repo_push", { path: currentRepo });
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Header />
      <div className="p-4">
        <div className="mb-4">
          <button className="px-3 py-1 bg-blue-600 text-white rounded mr-2" onClick={startAuth}>
            Sign in with GitHub
          </button>
          <button className="px-3 py-1 bg-gray-700 text-white rounded" onClick={openRepo}>
            Open Repository
          </button>
        </div>
        {accounts.length > 0 && (
          <div className="mb-4">
            <span className="text-sm">Signed in as {accounts[0].login}</span>
          </div>
        )}
        {currentRepo && (
          <div className="mb-4">
            <div className="flex gap-2 mb-2">
              <button className="px-3 py-1" onClick={() => setActiveTab("changes")}>Changes</button>
              <button className="px-3 py-1" onClick={() => setActiveTab("branches")}>Branches</button>
              <button className="px-3 py-1" onClick={() => setActiveTab("history")}>History</button>
              <button className="px-3 py-1" onClick={() => setActiveTab("remotes")}>Remotes</button>
              <button className="px-3 py-1" onClick={() => setActiveTab("stash")}>Stash</button>
              <button className="px-3 py-1" onClick={() => setActiveTab("tags")}>Tags</button>
            </div>
            <div className="flex gap-2 mb-4">
              <button className="px-3 py-1 bg-green-600 text-white rounded" onClick={commit}>Commit</button>
              <button className="px-3 py-1 bg-blue-600 text-white rounded" onClick={push}>Push</button>
            </div>
            {activeTab === "changes" && <div>Files: {files.length}</div>}
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
