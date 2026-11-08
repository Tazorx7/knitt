import { lazy, Suspense, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-shell";
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
import IssuesPanel from "./components/IssuesPanel";
import PRsPanel from "./components/PRsPanel";
import ActionsPanel from "./components/ActionsPanel";

// Monaco is large; load it only when a diff is actually opened.
const DiffViewer = lazy(() => import("./components/DiffViewer"));

type Account = {
  id: string;
  host: string;
  login: string;
  name?: string;
  avatar_url: string;
};

type RepoInfo = {
  path: string;
  name: string;
  branch?: string;
};

type FileStatus = {
  path: string;
  status: string;
  staged: boolean;
  unstaged: boolean;
};

// Log the raw error to the console for debugging, but never surface internals to the user.
function logError(context: string, e: unknown) {
  console.error(`[knitt:${context}]`, e);
}

// Map GitHub device-flow error codes to friendly, non-leaking messages.
function friendlyAuthError(code: string): string {
  switch (code) {
    case "access_denied":
      return "Authorization was denied.";
    case "expired_token":
      return "The code expired. Please try again.";
    case "incorrect_client_credentials":
    case "incorrect_device_code":
      return "Sign-in is not configured correctly.";
    default:
      return "Sign-in failed. Please try again.";
  }
}

function App() {
  const [isDark, setIsDark] = useState(false);
  const [collapsed, setCollapsed] = useState({
    changes: false,
    branches: false,
    history: false,
    remotes: false,
    stash: false,
    tags: false,
    issues: false,
    prs: false,
    actions: false,
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

  const [repo, setRepo] = useState<RepoInfo | null>(null);
  const [files, setFiles] = useState<FileStatus[]>([]);
  const [commitMsg, setCommitMsg] = useState("");
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileContents, setFileContents] = useState<{ original: string; modified: string } | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

  useEffect(() => {
    invoke<boolean>("auth_check_secret_service").catch(() => {});
    refreshAccounts();
    
    const unlisten1 = listen("auth:code", (e: any) => {
      setUserCode(e.payload.user_code);
      setVerificationUri(e.payload.verification_uri);
      setStatus("Enter the code in your browser");
    });
    const unlisten2 = listen("auth:pending", () => setStatus("Waiting for authorization..."));
    const unlisten3 = listen("auth:success", (_e: any) => {
      setStatus("Signed in successfully");
      setShowSignIn(false);
      setUserCode(null);
      setVerificationUri(null);
      refreshAccounts();
    });
    const unlisten4 = listen("auth:error", (e: any) => {
      const code = e?.payload?.error ?? "";
      logError("auth", e);
      setStatus(friendlyAuthError(code));
    });
    const unlisten5 = listen("auth:expired", () => setStatus("Code expired, try again"));
    
    return () => {
      unlisten1.then(f => f());
      unlisten2.then(f => f());
      unlisten3.then(f => f());
      unlisten4.then(f => f());
      unlisten5.then(f => f());
    };
  }, []);

  async function refreshAccounts() {
    try {
      const res: any = await invoke("auth_list_accounts");
      setAccounts(res.accounts || []);
      setActiveId(res.active_account_id || null);
    } catch {}
  }

  async function startSignIn() {
    const h = host === "enterprise" ? customHost.trim() : "github.com";
    if (host === "enterprise" && !h) return;
    setStatus("Requesting device code...");
    setUserCode(null);
    setVerificationUri(null);
    try {
      await invoke("auth_start_device_flow", { host: h });
      poll();
    } catch (e: any) {
      logError("startSignIn", e);
      // The backend returns clean, user-facing messages for auth failures.
      const msg = typeof e === "string" ? e : undefined;
      setStatus(msg ?? "Sign-in failed. Please try again.");
    }
  }

  async function poll() {
    try {
      const done = await invoke<boolean>("auth_poll_device_flow");
      if (!done) {
        setTimeout(poll, 100);
      }
    } catch {
      setTimeout(poll, 1000);
    }
  }

  async function cancel() {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("auth_cancel_device_flow");
    } catch (e) {
      console.error(e);
    }
    setShowSignIn(false);
    setUserCode(null);
    setVerificationUri(null);
    setStatus("");
  }

  async function signOut(id: string) {
    await invoke("auth_sign_out", { accountId: id });
    refreshAccounts();
  }

  async function openRepo() {
    const selected = await openDialog({
      directory: true,
      multiple: false,
    });
    if (typeof selected === "string") {
      try {
        const info: RepoInfo = await invoke("repo_open", { path: selected });
        setRepo(info);
        refreshStatus(selected);
      } catch (e: any) {
        logError("openRepo", e);
        setStatus("Could not open that folder.");
      }
    }
  }

  async function refreshStatus(path: string) {
    try {
      const status: FileStatus[] = await invoke("repo_status", { path });
      setFiles(status);
      // Drop the selection if the file no longer has changes.
      setSelectedFile((cur) => (cur && status.some((f) => f.path === cur) ? cur : null));
    } catch (e: any) {
      logError("refreshStatus", e);
      setStatus("Could not load repository status.");
    }
  }

  async function openFileDiff(file: string) {
    if (!repo) return;
    setSelectedFile(file);
    setDiffLoading(true);
    setFileContents(null);
    try {
      const res = await invoke<{ original: string; modified: string }>("repo_file_contents", {
        path: repo.path,
        file,
      });
      setFileContents(res);
    } catch (e) {
      logError("openFileDiff", e);
      setStatus("Could not load file diff.");
      setFileContents({ original: "", modified: "" });
    } finally {
      setDiffLoading(false);
    }
  }

  async function commit() {
    if (!repo || !commitMsg.trim()) return;
    try {
      await invoke("repo_commit", { path: repo.path, message: commitMsg });
      setCommitMsg("");
      refreshStatus(repo.path);
      setStatus("Committed");
    } catch (e: any) {
      logError("commit", e);
      setStatus("Commit failed.");
    }
  }

  async function push() {
    if (!repo) return;
    try {
      await invoke("repo_push", { path: repo.path });
      setStatus("Pushed");
    } catch (e: any) {
      logError("push", e);
      setStatus("Push failed.");
    }
  }

  async function fetchRepo() {
    if (!repo) return;
    try {
      await invoke("repo_fetch", { path: repo.path });
      setStatus("Fetched");
    } catch (e: any) {
      logError("fetch", e);
      setStatus("Fetch failed.");
    }
  }

  const active = accounts.find(a => a.id === activeId);

  return (
    <div className="min-h-screen bg-neutral-50 text-neutral-900 dark:bg-neutral-950 dark:text-neutral-50 relative overflow-hidden">
      <img src={isDark ? darkLogo : logo} alt="" className="absolute inset-0 m-auto w-72 h-72 opacity-5 pointer-events-none" />
      <header className="flex items-center gap-4 border-b border-neutral-200 px-4 py-2 dark:border-neutral-800 relative z-10">
        <div className="flex shrink-0 items-center gap-2">
          <img src={isDark ? darkLogo : logo} alt="knitt" className="w-6 h-6" />
          <span className="font-semibold">knitt</span>
        </div>
        <div className="min-w-0 flex-1">
          {repo && (
            <div
              className="truncate text-sm opacity-80"
              title={`${repo.name}${repo.branch ? ` • ${repo.branch}` : ""}`}
            >
              {repo.name} {repo.branch ? `• ${repo.branch}` : ""}
            </div>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {repo && (
            <button
              onClick={push}
              className="rounded px-2 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
            >
              Push
            </button>
          )}
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
                      <button
                        key={i}
                        type="button"
                        onClick={() => openFileDiff(f.path)}
                        aria-current={selectedFile === f.path ? "true" : undefined}
                        className={`w-full rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 ${
                          selectedFile === f.path ? "bg-neutral-100 dark:bg-neutral-900 font-medium" : ""
                        }`}
                      >
                        <div className="truncate">{f.path}</div>
                      </button>
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

              {active && (
                <>
                  <div className="mb-2">
                    <button
                      type="button"
                      className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                      aria-expanded={!collapsed.issues}
                      onClick={() => setCollapsed((c) => ({ ...c, issues: !c.issues }))}
                    >
                      <span>Issues</span>
                      <span>{collapsed.issues ? "▸" : "▾"}</span>
                    </button>
                    {!collapsed.issues && <IssuesPanel repoPath={repo.path} />}
                  </div>

                  <div className="mb-2">
                    <button
                      type="button"
                      className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                      aria-expanded={!collapsed.prs}
                      onClick={() => setCollapsed((c) => ({ ...c, prs: !c.prs }))}
                    >
                      <span>Pull Requests</span>
                      <span>{collapsed.prs ? "▸" : "▾"}</span>
                    </button>
                    {!collapsed.prs && <PRsPanel repoPath={repo.path} />}
                  </div>

                  <div className="mb-2">
                    <button
                      type="button"
                      className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded px-1"
                      aria-expanded={!collapsed.actions}
                      onClick={() => setCollapsed((c) => ({ ...c, actions: !c.actions }))}
                    >
                      <span>Actions</span>
                      <span>{collapsed.actions ? "▸" : "▾"}</span>
                    </button>
                    {!collapsed.actions && <ActionsPanel repoPath={repo.path} />}
                  </div>
                </>
              )}
            </>
          )}
        </aside>
        <section className="flex-1 flex flex-col overflow-hidden">
          {status && <div className="px-4 pt-2 text-xs opacity-70">{status}</div>}
          {selectedFile ? (
            <>
              <div className="flex items-center justify-between border-b border-neutral-200 px-4 py-2 dark:border-neutral-800">
                <span className="truncate text-sm font-medium">{selectedFile}</span>
                <button
                  type="button"
                  onClick={() => { setSelectedFile(null); setFileContents(null); }}
                  className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
                  aria-label="Close diff"
                >
                  ✕
                </button>
              </div>
              <div className="min-h-0 flex-1">
                {diffLoading ? (
                  <div className="p-4 text-sm opacity-60">Loading diff…</div>
                ) : fileContents ? (
                  <Suspense fallback={<div className="p-4 text-sm opacity-60">Loading editor…</div>}>
                    <DiffViewer
                      original={fileContents.original}
                      modified={fileContents.modified}
                      fileName={selectedFile}
                    />
                  </Suspense>
                ) : null}
              </div>
            </>
          ) : (
            <div className="p-4">
              {active ? (
                <div className="mt-2 flex items-center gap-2 text-sm">
                  <img src={active.avatar_url} className="h-6 w-6 rounded-full" alt={active.login} />
                  <span>{active.login}</span>
                  <button onClick={() => signOut(active.id)} className="rounded px-2 py-1 hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500">Sign out</button>
                </div>
              ) : (
                <div className="mt-2 text-sm opacity-60">
                  {repo ? "Select a changed file to view its diff." : "Open a repository to get started."}
                </div>
              )}
            </div>
          )}
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
