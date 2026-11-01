import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";


import { open as openDialog } from "@tauri-apps/plugin-dialog";
import logo from "./assets/logo.png";
import darkLogo from "./assets/dark-logo.jpg";
import ThemeToggle from "./components/ThemeToggle";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import BranchPanel from "./components/BranchPanel";
import HistoryPanel from "./components/HistoryPanel";
import RemotesPanel from "./components/RemotesPanel";
import StashPanel from "./components/StashPanel";
import TagsPanel from "./components/TagsPanel";

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
  const [isDark, setIsDark] = useState(false);
  const [collapsed, setCollapsed] = useState({
    changes: false,
    branches: false,
    history: false,
    remotes: false,
    stash: false,
    tags: false,
  });

  useEffect(() => {
    const updateTheme = () => {
      const root = document.documentElement;
      if (root.classList.contains("dark")) {
        setIsDark(true);
      } else if (root.classList.contains("light")) {
        setIsDark(false);
      } else {
        const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        setIsDark(prefersDark);
        root.classList.add(prefersDark ? "dark" : "light");
      }
    };
    updateTheme();
    const observer = new MutationObserver(updateTheme);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", updateTheme);
    return () => {
      observer.disconnect();
      mediaQuery.removeEventListener("change", updateTheme);
    };
  }, []);

  useKeyboardShortcuts({
    onOpenRepo: openRepo,
    onCommit: commit,
    onPush: push,
    onFetch: fetchRepo,
  });
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [showSignIn, setShowSignIn] = useState(false);
  const [host, setHost] = useState("github.com");
  const [customHost, setCustomHost] = useState("");
  const [userCode, setUserCode] = useState<string | null>(null);
  const [verificationUri, setVerificationUri] = useState<string | null>(null);
  const [status, setStatus] = useState<string>("");
  
  const [accounts, setAccounts] = useState<Account[]>([]);
  
  const [currentRepo, setCurrentRepo] = useState<string | null>(null);
  const [files, setFiles] = useState<FileStatus[]>([]);
  
  
  const [activeTab, setActiveTab] = useState("changes");

  useEffect(() => {
    invoke<boolean>("auth_check_secret_service").catch(() => {});
    refreshAccounts();
    
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
        <div className="flex items-center gap-2">
          <ThemeToggle />
          {!active ? (
            <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); setShowSignIn(true); }} className="rounded px-2 py-1 text-xs bg-blue-600 text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500">Sign in with GitHub</button>
          ) : (
            <div className="flex items-center gap-2">
              <img src={active.avatar_url} alt={active.login} className="h-5 w-5 rounded-full" />
              <span className="text-xs">{active.login}</span>
            </div>
          )}
        </div>
        {repo && (
          <>
            <div className="text-sm opacity-80">{repo.name} {repo.branch ? `• ${repo.branch}` : ""}</div>
            <button onClick={push} className="rounded px-2 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500">Push</button>
          </>
        )}
      </header>
      <main className="flex h-[calc(100vh-3rem)] relative z-10">
        <aside className="w-80 border-r border-neutral-200 p-3 dark:border-neutral-800 overflow-y-auto" role="navigation" aria-label="Repository panels">
          <div className="mb-2">
            <button
              type="button"
              className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
              aria-expanded={!collapsed.changes}
              onClick={() => setCollapsed((c) => ({ ...c, changes: !c.changes }))}
            >
              <span>Changes</span>
              <span>{collapsed.changes ? "▸" : "▾"}</span>
            </button>
            {!collapsed.changes && (
              <>
                {files.length === 0 ? (
                  <div className="text-sm opacity-60">No changes</div>
                ) : (
                  <div className="flex flex-col gap-1">
                    {files.map((f, i) => (
                      <div key={i} className="rounded px-2 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-900">
                        <div className="truncate">{f.path}</div>
                      </div>
                    ))}
                  </div>
                )}
                {repo && (
                  <div className="mt-4 flex flex-col gap-2">
                    <textarea
                      value={commitMsg}
                      onChange={(e) => setCommitMsg(e.target.value)}
                      placeholder="Commit message"
                      className="rounded border border-neutral-300 p-2 text-sm dark:border-neutral-700 dark:bg-neutral-900"
                      rows={3}
                      aria-label="Commit message"
                    />
                    <button onClick={commit} className="rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500">Commit</button>
                  </div>
                )}
              </>
            )}
          </div>

          {repo && (
            <>
              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                  aria-expanded={!collapsed.branches}
                  onClick={() => setCollapsed((c) => ({ ...c, branches: !c.branches }))}
                >
                  <span>Branches</span>
                  <span>{collapsed.branches ? "▸" : "▾"}</span>
                </button>
                {!collapsed.branches && <BranchPanel repoPath={repo.path} />}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                  aria-expanded={!collapsed.history}
                  onClick={() => setCollapsed((c) => ({ ...c, history: !c.history }))}
                >
                  <span>History</span>
                  <span>{collapsed.history ? "▸" : "▾"}</span>
                </button>
                {!collapsed.history && <HistoryPanel repoPath={repo.path} />}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                  aria-expanded={!collapsed.remotes}
                  onClick={() => setCollapsed((c) => ({ ...c, remotes: !c.remotes }))}
                >
                  <span>Remotes</span>
                  <span>{collapsed.remotes ? "▸" : "▾"}</span>
                </button>
                {!collapsed.remotes && <RemotesPanel repoPath={repo.path} />}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                  aria-expanded={!collapsed.stash}
                  onClick={() => setCollapsed((c) => ({ ...c, stash: !c.stash }))}
                >
                  <span>Stash</span>
                  <span>{collapsed.stash ? "▸" : "▾"}</span>
                </button>
                {!collapsed.stash && <StashPanel repoPath={repo.path} />}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                  aria-expanded={!collapsed.tags}
                  onClick={() => setCollapsed((c) => ({ ...c, tags: !c.tags }))}
                >
                  <span>Tags</span>
                  <span>{collapsed.tags ? "▸" : "▾"}</span>
                </button>
                {!collapsed.tags && <TagsPanel repoPath={repo.path} />}
              </div>
            </>
          )}
        </aside>
        <section className="flex-1 p-4">
          {status && <div className="mt-2 text-xs opacity-70">{status}</div>}
          {active ? (
            <div className="mt-4 flex items-center gap-2 text-sm">
              <img src={active.avatar_url} className="h-6 w-6 rounded-full" />
              <span>{active.login}</span>
              <button onClick={() => signOut(active.id)} className="rounded px-2 py-1 hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500">Sign out</button>
            </div>
          ) : null}
        </section>
      </main>

      {showSignIn && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50 z-[100]" role="dialog" aria-modal="true" onClick={(e) => { if (e.target === e.currentTarget) { cancel(); setShowSignIn(false); } }}>
          <div className="w-[420px] rounded-lg border border-neutral-200 bg-neutral-50 p-6 shadow-lg dark:border-neutral-800 dark:bg-neutral-900">
            {!userCode ? (
              <>
                <h2 className="text-lg font-medium">Sign in to GitHub</h2>
                <div className="mt-4 flex flex-col gap-2">
                  <label className="flex items-center gap-2 text-sm">
                    <input type="radio" checked={host === "github.com"} onChange={() => setHost("github.com")} />
                    GitHub.com
                  </label>
                  <label className="flex items-center gap-2 text-sm">
                    <input type="radio" checked={host === "enterprise"} onChange={() => setHost("enterprise")} />
                    GitHub Enterprise Server
                  </label>
                  {host === "enterprise" && (
                    <input
                      value={customHost}
                      onChange={(e) => setCustomHost(e.target.value)}
                      placeholder="git.example.com"
                      className="mt-1 rounded border border-neutral-300 px-2 py-1 text-sm dark:border-neutral-700 dark:bg-neutral-800"
                    />
                  )}
                </div>
                <div className="mt-6 flex justify-end gap-2">
                  <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); cancel(); }} className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Cancel</button>
                  <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); startSignIn(); }} className="rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200">Continue</button>
                </div>
                {status && <div className="mt-3 text-xs opacity-70">{status}</div>}
              </>
            ) : (
              <>
                <h2 className="text-lg font-medium">Authorize this app</h2>
                <p className="mt-2 text-sm opacity-80">Enter this code in your browser</p>
                <div className="mt-4 flex items-center justify-center rounded bg-neutral-100 py-3 text-2xl font-mono tracking-widest dark:bg-neutral-800">{userCode}</div>
                <div className="mt-4 flex flex-col gap-2">
                  <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); verificationUri && open(verificationUri); }} className="rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200">Open in browser</button>
                  <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); verificationUri && navigator.clipboard.writeText(verificationUri); }} className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Copy verification URL</button>
                </div>
                <div className="mt-4 text-xs opacity-70">{status}</div>
                <div className="mt-4 flex justify-end">
                  <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); cancel(); }} className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Cancel</button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
      <button
        type="button"
        className="fixed bottom-4 right-4 w-12 h-12 bg-gray-700 text-white rounded-full text-2xl shadow-lg z-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
        title="Open Repository"
        aria-label="Open Repository"
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          openRepo();
        }}
      >
        +
      </button>
    </div>
  );
}

export default App;
