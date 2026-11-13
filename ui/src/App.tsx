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
import ComparePanel from "./components/ComparePanel";
import CommitDiffDialog from "./components/CommitDiffDialog";
import RemotesPanel from "./components/RemotesPanel";
import StashPanel from "./components/StashPanel";
import TagsPanel from "./components/TagsPanel";
import IssuesPanel from "./components/IssuesPanel";
import PRsPanel from "./components/PRsPanel";
import ActionsPanel from "./components/ActionsPanel";
import SettingsView from "./components/settings/SettingsView";
import ConfirmDialog from "./components/ConfirmDialog";
import NewRepoDialog from "./components/NewRepoDialog";
import CloneDialog from "./components/CloneDialog";
import {
  loadSettings,
  resetSettings,
  saveSettings,
  settingsDefaults,
  type Settings,
} from "./lib/settings";
import { applyTheme, resolveDark } from "./lib/theme";
import {
  listAccounts,
  setActiveAccount,
  signOutAccount,
  type Account,
} from "./lib/accounts";

// Monaco is large; load it only when a diff is actually opened.
const DiffViewer = lazy(() => import("./components/DiffViewer"));

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
  tracked: boolean;
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
  const [settings, setSettings] = useState<Settings>(settingsDefaults);
  const [view, setView] = useState<"main" | "settings">("main");
  const [collapsed, setCollapsed] = useState({
    changes: false,
    branches: false,
    history: false,
    compare: true,
    remotes: false,
    stash: false,
    tags: false,
    issues: false,
    prs: false,
    actions: false,
  });

  useEffect(() => {
    const observer = new MutationObserver(() => {
      setIsDark(document.documentElement.classList.contains("dark"));
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  // Apply the persisted theme whenever it changes.
  useEffect(() => {
    applyTheme(settings.theme);
  }, [settings.theme]);

  // Accent is a CSS variable on the document root so focus rings and primary
  // actions pick it up everywhere, including portalled dialogs.
  useEffect(() => {
    document.documentElement.dataset.accent = settings.accent;
  }, [settings.accent]);

  // Follow desktop light/dark changes while in system mode.
  useEffect(() => {
    if (settings.theme !== "system") return;
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => applyTheme("system");
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, [settings.theme]);

  // Load settings once, then restore the last repository if the user opted in.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const loaded = await loadSettings();
        if (cancelled) return;
        setSettings(loaded);
        applyTheme(loaded.theme);
        if (
          loaded.restore_last_repository &&
          loaded.startup_view === "last" &&
          loaded.last_repository_path
        ) {
          try {
            const info = await invoke<RepoInfo>("repo_open", {
              path: loaded.last_repository_path,
            });
            if (cancelled) return;
            setRepo(info);
            refreshStatus(info.path);
          } catch {
            // The saved repository moved or was deleted; start on the empty view.
          }
        }
      } catch (e) {
        logError("loadSettings", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useKeyboardShortcuts({
    onOpenRepo: openRepo,
    onCommit: commit,
    onPush: push,
    onFetch: fetchRepo,
    onOpenSettings: () => setView("settings"),
    enabled: settings.keyboard_shortcuts,
  });
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [showSignIn, setShowSignIn] = useState(false);
  const [showNewRepo, setShowNewRepo] = useState(false);
  const [showCloneRepo, setShowCloneRepo] = useState(false);
  const [amendMode, setAmendMode] = useState(false);
  const [undoCommit, setUndoCommit] = useState(false);
  const [undoBusy, setUndoBusy] = useState(false);
  const [undoError, setUndoError] = useState<string | null>(null);
  const [showOpenMenu, setShowOpenMenu] = useState(false);
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
  const [discardTarget, setDiscardTarget] = useState<string | null>(null);
  const [discardBusy, setDiscardBusy] = useState(false);
  const [discardError, setDiscardError] = useState<string | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [commitDetailId, setCommitDetailId] = useState<string | null>(null);

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
      const res = await listAccounts();
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
    try {
      await signOutAccount(id);
      refreshAccounts();
    } catch (e) {
      logError("signOut", e);
      setStatus("Could not sign out.");
    }
  }

  async function activateAccount(id: string) {
    try {
      await setActiveAccount(id);
      setActiveId(id);
    } catch (e) {
      logError("activateAccount", e);
      setStatus("Could not switch accounts.");
    }
  }

  /** Opens the existing sign-in dialog pre-targeted at `host`. */
  function requestSignIn(targetHost: string) {
    if (targetHost === "github.com") {
      setHost("github.com");
      setCustomHost("");
    } else {
      setHost("enterprise");
      setCustomHost(targetHost);
    }
    setUserCode(null);
    setVerificationUri(null);
    setShowSignIn(true);
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

  /** Shows a repository produced by the New/Clone dialogs and refreshes the
   *  recent list that the backend just updated. */
  async function adoptRepo(info: RepoInfo, message: string) {
    setRepo(info);
    setAmendMode(false);
    setShowNewRepo(false);
    setShowCloneRepo(false);
    setSelectedFile(null);
    setFileContents(null);
    setStatus(message);
    refreshStatus(info.path);
    try {
      setSettings(await loadSettings());
    } catch (e) {
      logError("loadSettings", e);
    }
  }

  function repoCreated(info: RepoInfo) {
    adoptRepo(info, `Created ${info.name}.`);
  }

  function repoCloned(info: RepoInfo) {
    adoptRepo(info, `Cloned ${info.name}.`);
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

  /** Re-reads repository info after a checkout so the header, changes and
   *  history all reflect the new branch. */
  async function refreshRepo() {
    if (!repo) return;
    try {
      const info: RepoInfo = await invoke("repo_open", { path: repo.path });
      setRepo(info);
      setAmendMode(false);
      setSelectedFile(null);
      setFileContents(null);
      await refreshStatus(repo.path);
    } catch (e: any) {
      logError("refreshRepo", e);
      setStatus("Could not refresh the repository.");
    }
  }

  /** Shows a pre-computed diff, used by the branch comparison panel. */
  function openCompareDiff(file: string, contents: { original: string; modified: string }) {
    setDiffLoading(false);
    setSelectedFile(file);
    setFileContents(contents);
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

  async function stagePaths(paths: string[]) {
    if (!repo || paths.length === 0) return;
    try {
      await invoke("repo_stage_files", { path: repo.path, files: paths });
      refreshStatus(repo.path);
    } catch (e) {
      logError("stageFiles", e);
      setStatus("Could not stage those changes.");
    }
  }

  async function unstagePaths(paths: string[]) {
    if (!repo || paths.length === 0) return;
    try {
      await invoke("repo_unstage_files", { path: repo.path, files: paths });
      refreshStatus(repo.path);
    } catch (e) {
      logError("unstageFiles", e);
      setStatus("Could not unstage those changes.");
    }
  }

  async function stageEverything() {
    if (!repo) return;
    try {
      await invoke("repo_stage_all", { path: repo.path });
      refreshStatus(repo.path);
    } catch (e) {
      logError("stageAll", e);
      setStatus("Could not stage the changes.");
    }
  }

  /** Enters amend mode with the last commit's message, or leaves it. */
  async function toggleAmend() {
    if (amendMode) {
      setAmendMode(false);
      setCommitMsg("");
      return;
    }
    if (!repo) return;
    try {
      const commits = await invoke<any[]>("repo_log", { path: repo.path, limit: 1 });
      const head = Array.isArray(commits) ? commits[0] : null;
      if (!head) {
        setStatus("There is no commit to amend.");
        return;
      }
      setCommitMsg(head.message ?? "");
      setAmendMode(true);
    } catch (e) {
      logError("amend", e);
      setStatus("Could not read the last commit.");
    }
  }

  async function undoLastCommit() {
    if (!repo) return;
    setUndoBusy(true);
    setUndoError(null);
    try {
      await invoke("repo_undo_last_commit", { path: repo.path });
      setUndoCommit(false);
      setAmendMode(false);
      setStatus("Undid the last commit. Its changes are staged.");
      refreshStatus(repo.path);
    } catch (e) {
      logError("undoCommit", e);
      setUndoError(typeof e === "string" ? e : "Could not undo the last commit.");
    } finally {
      setUndoBusy(false);
    }
  }

  async function commit() {
    if (!repo || !commitMsg.trim()) return;
    try {
      if (amendMode) {
        await invoke("repo_amend_commit", { path: repo.path, message: commitMsg });
        setStatus("Amended the last commit.");
      } else {
        await invoke("repo_commit", { path: repo.path, message: commitMsg });
        setStatus("Committed");
      }
      setAmendMode(false);
      setCommitMsg("");
      refreshStatus(repo.path);
    } catch (e: any) {
      logError("commit", e);
      setStatus(
        typeof e === "string" && e.length > 0
          ? e
          : settings.signing_mode === "off"
            ? "Commit failed."
            : "Commit failed. Check the signing key and tool.",
      );
    }
  }

  async function discardFile(file: string) {
    if (!repo) return;
    setDiscardBusy(true);
    setDiscardError(null);
    try {
      await invoke("repo_discard", { path: repo.path, files: [file] });
      setDiscardTarget(null);
      setSelectedFile(null);
      setFileContents(null);
      await refreshStatus(repo.path);
    } catch {
      setDiscardError("Could not discard that file's changes.");
    } finally {
      setDiscardBusy(false);
    }
  }

  function requestDiscard(file: string) {
    if (settings.confirm_discard) {
      setDiscardError(null);
      setDiscardTarget(file);
    } else {
      discardFile(file);
    }
  }

  async function push() {
    if (!repo) return;
    try {
      await invoke("repo_push", { path: repo.path, mode: settings.push_mode });
      setStatus("Pushed");
    } catch (e: any) {
      logError("push", e);
      setStatus("Push failed.");
    }
  }

  async function fetchRepo() {
    if (!repo) return;
    try {
      await invoke("repo_fetch", {
        path: repo.path,
        name: "origin",
        prune: settings.fetch_prune,
      });
      setStatus("Fetched");
    } catch (e: any) {
      logError("fetch", e);
      setStatus("Fetch failed.");
    }
  }

  async function updateSettings(patch: Partial<Settings>) {
    const next = { ...settings, ...patch };
    // Apply appearance immediately, but only trust the backend's response for
    // what actually got persisted.
    setSettings(next);
    if (patch.theme) applyTheme(patch.theme);
    try {
      const saved = await saveSettings(next);
      setSettings(saved);
      applyTheme(saved.theme);
    } catch (e) {
      logError("updateSettings", e);
      setStatus("Could not save settings.");
    }
  }

  async function resetSettingsToDefaults() {
    const saved = await resetSettings();
    setSettings(saved);
    applyTheme(saved.theme);
  }

  function toggleTheme() {
    updateSettings({ theme: resolveDark(settings.theme) ? "light" : "dark" });
  }

  const active = accounts.find(a => a.id === activeId);
  const stagedFiles = files.filter((f) => f.staged);
  const unstagedFiles = files.filter((f) => f.unstaged);

  return (
    <div className={`min-h-screen bg-neutral-50 text-neutral-900 dark:bg-neutral-950 dark:text-neutral-50 relative overflow-hidden${settings.reduce_motion ? " reduce-motion" : ""}${
          settings.density === "compact" ? " density-compact" : ""
        }`}>
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
              className="rounded px-2 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Push
            </button>
          )}
          <ThemeToggle dark={isDark} onToggle={toggleTheme} />
          <button
            type="button"
            onClick={() => setView(view === "settings" ? "main" : "settings")}
            className="rounded px-2 py-1 text-xs bg-neutral-200 dark:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            title="Settings (Ctrl+,)"
            aria-label="Settings"
            aria-pressed={view === "settings"}
          >
            ⚙️
          </button>
          {!active ? (
            <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); setShowSignIn(true); }} className="rounded px-2 py-1 text-xs bg-[var(--accent)] text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]">Sign in with GitHub</button>
          ) : (
            <div className="flex items-center gap-2">
              <img src={active.avatar_url} alt={active.login} className="h-5 w-5 rounded-full" />
              <span className="text-xs">{active.login}</span>
            </div>
          )}
        </div>
      </header>
      <main className="flex h-[calc(100vh-3rem)] relative z-10">
        {view === "settings" ? (
          <SettingsView
            settings={settings}
            onChange={updateSettings}
            onClose={() => setView("main")}
            onReset={resetSettingsToDefaults}
            repoPath={repo?.path ?? null}
            accounts={{
              accounts,
              activeId,
              onSignIn: requestSignIn,
              onSignOut: signOut,
              onSetActive: activateAccount,
            }}
          />
        ) : (
          <>
        <aside className="w-80 border-r border-neutral-200 p-3 dark:border-neutral-800 overflow-y-auto" role="navigation" aria-label="Repository panels">
          <div className="mb-2">
            <button
              type="button"
              className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
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
                  <div className="flex flex-col gap-3">
                    {stagedFiles.length > 0 && (
                      <section aria-label="Staged changes">
                        <div className="flex items-center justify-between pb-1 text-xs opacity-70">
                          <span>Staged ({stagedFiles.length})</span>
                          <button
                            type="button"
                            onClick={() => unstagePaths(stagedFiles.map((f) => f.path))}
                            className="rounded px-1 hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                          >
                            Unstage all
                          </button>
                        </div>
                        <div className="flex flex-col gap-1">
                          {stagedFiles.map((f) => (
                            <div key={`staged:${f.path}`} className="flex items-center gap-1">
                              <button
                                type="button"
                                onClick={() => openFileDiff(f.path)}
                                aria-current={selectedFile === f.path ? "true" : undefined}
                                className={`min-w-0 flex-1 rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
                                  selectedFile === f.path ? "bg-neutral-100 dark:bg-neutral-900 font-medium" : ""
                                }`}
                              >
                                <div className="truncate">{f.path}</div>
                              </button>
                              <button
                                type="button"
                                onClick={() => unstagePaths([f.path])}
                                aria-label={`Unstage ${f.path}`}
                                className="shrink-0 rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                              >
                                Unstage
                              </button>
                            </div>
                          ))}
                        </div>
                      </section>
                    )}

                    {unstagedFiles.length > 0 && (
                      <section aria-label="Unstaged changes">
                        <div className="flex items-center justify-between pb-1 text-xs opacity-70">
                          <span>Changes ({unstagedFiles.length})</span>
                          <button
                            type="button"
                            onClick={stageEverything}
                            className="rounded px-1 hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                          >
                            Stage all
                          </button>
                        </div>
                        <div className="flex flex-col gap-1">
                          {unstagedFiles.map((f) => (
                            <div key={`unstaged:${f.path}`} className="flex items-center gap-1">
                              <button
                                type="button"
                                onClick={() => openFileDiff(f.path)}
                                aria-current={selectedFile === f.path ? "true" : undefined}
                                className={`min-w-0 flex-1 rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
                                  selectedFile === f.path ? "bg-neutral-100 dark:bg-neutral-900 font-medium" : ""
                                }`}
                              >
                                <div className="truncate">{f.path}</div>
                              </button>
                              <button
                                type="button"
                                onClick={() => stagePaths([f.path])}
                                aria-label={`Stage ${f.path}`}
                                className="shrink-0 rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                              >
                                Stage
                              </button>
                              {f.tracked && (
                                <button
                                  type="button"
                                  onClick={() => requestDiscard(f.path)}
                                  aria-label={`Discard changes to ${f.path}`}
                                  className="shrink-0 rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                                >
                                  Discard
                                </button>
                              )}
                            </div>
                          ))}
                        </div>
                      </section>
                    )}
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
                    <div className="flex flex-wrap items-center gap-2">
                      <button
                        onClick={commit}
                        disabled={
                          (!amendMode && stagedFiles.length === 0) || !commitMsg.trim()
                        }
                        title={
                          !amendMode && stagedFiles.length === 0
                            ? "Stage changes to commit"
                            : undefined
                        }
                        className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                      >
                        {amendMode ? "Amend commit" : "Commit"}
                      </button>
                      <button
                        type="button"
                        onClick={toggleAmend}
                        className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                      >
                        {amendMode ? "Cancel amend" : "Amend last"}
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          setUndoError(null);
                          setUndoCommit(true);
                        }}
                        className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                      >
                        Undo last commit
                      </button>
                    </div>
                    {amendMode ? (
                      <p className="text-xs opacity-60">
                        Amending the last commit with the staged changes.
                      </p>
                    ) : (
                      stagedFiles.length === 0 && (
                        <p className="text-xs opacity-60">Stage changes to commit.</p>
                      )
                    )}
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
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
                  aria-expanded={!collapsed.branches}
                  onClick={() => setCollapsed((c) => ({ ...c, branches: !c.branches }))}
                >
                  <span>Branches</span>
                  <span>{collapsed.branches ? "▸" : "▾"}</span>
                </button>
                {!collapsed.branches && (
                  <BranchPanel
                    repoPath={repo.path}
                    currentBranch={repo.branch}
                    onCheckedOut={refreshRepo}
                  />
                )}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
                  aria-expanded={!collapsed.history}
                  onClick={() => setCollapsed((c) => ({ ...c, history: !c.history }))}
                >
                  <span>History</span>
                  <span>{collapsed.history ? "▸" : "▾"}</span>
                </button>
                {!collapsed.history && (
                  <HistoryPanel
                    key={repo.branch ?? repo.path}
                    repoPath={repo.path}
                    onSelectCommit={setCommitDetailId}
                  />
                )}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
                  aria-expanded={!collapsed.compare}
                  onClick={() => setCollapsed((c) => ({ ...c, compare: !c.compare }))}
                >
                  <span>Compare</span>
                  <span>{collapsed.compare ? "▸" : "▾"}</span>
                </button>
                {!collapsed.compare && (
                  <ComparePanel
                    repoPath={repo.path}
                    currentBranch={repo.branch}
                    onOpenDiff={openCompareDiff}
                  />
                )}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
                  aria-expanded={!collapsed.remotes}
                  onClick={() => setCollapsed((c) => ({ ...c, remotes: !c.remotes }))}
                >
                  <span>Remotes</span>
                  <span>{collapsed.remotes ? "▸" : "▾"}</span>
                </button>
                {!collapsed.remotes && (
                  <RemotesPanel
                    repoPath={repo.path}
                    pullStrategy={settings.pull_strategy}
                    fetchPrune={settings.fetch_prune}
                  />
                )}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
                  aria-expanded={!collapsed.stash}
                  onClick={() => setCollapsed((c) => ({ ...c, stash: !c.stash }))}
                >
                  <span>Stash</span>
                  <span>{collapsed.stash ? "▸" : "▾"}</span>
                </button>
                {!collapsed.stash && (
                  <StashPanel repoPath={repo.path} confirmDrop={settings.confirm_stash_drop} />
                )}
              </div>

              <div className="mb-2">
                <button
                  type="button"
                  className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
                  aria-expanded={!collapsed.tags}
                  onClick={() => setCollapsed((c) => ({ ...c, tags: !c.tags }))}
                >
                  <span>Tags</span>
                  <span>{collapsed.tags ? "▸" : "▾"}</span>
                </button>
                {!collapsed.tags && (
                  <TagsPanel repoPath={repo.path} confirmDelete={settings.confirm_tag_delete} />
                )}
              </div>

              {active && (
                <>
                  <div className="mb-2">
                    <button
                      type="button"
                      className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
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
                      className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
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
                      className="w-full flex items-center justify-between text-sm font-medium mb-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] rounded px-1"
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
                  className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
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
                      tabSize={settings.diff_tab_width}
                      sideBySide={settings.diff_side_by_side}
                      ignoreWhitespace={settings.diff_ignore_whitespace}
                      wrap={settings.diff_wrap}
                      fontSize={settings.diff_font_size}
                      dark={isDark}
                    />
                  </Suspense>
                ) : null}
              </div>
            </>
          ) : (
            <div className="p-4">
              <div className="mt-2 text-sm opacity-60">
                {repo ? "Select a changed file to view its diff." : "Open a repository to get started."}
              </div>
            </div>
          )}
        </section>
          </>
        )}

      {showNewRepo && (
        <NewRepoDialog
          defaultBranch={settings.default_branch}
          onCreated={repoCreated}
          onClose={() => setShowNewRepo(false)}
        />
      )}
      {showCloneRepo && (
        <CloneDialog
          onCloned={repoCloned}
          onClose={() => setShowCloneRepo(false)}
        />
      )}
      {repo && commitDetailId && (
        <CommitDiffDialog
          repoPath={repo.path}
          oid={commitDetailId}
          dark={isDark}
          tabSize={settings.diff_tab_width}
          sideBySide={settings.diff_side_by_side}
          ignoreWhitespace={settings.diff_ignore_whitespace}
          wrap={settings.diff_wrap}
          fontSize={settings.diff_font_size}
          onClose={() => setCommitDetailId(null)}
        />
      )}
      {undoCommit && (
        <ConfirmDialog
          title="Undo the last commit?"
          description="The commit is removed from the branch. Its changes stay staged and your files are untouched."
          confirmLabel="Undo commit"
          busy={undoBusy}
          error={undoError}
          onConfirm={undoLastCommit}
          onCancel={() => {
            setUndoCommit(false);
            setUndoError(null);
          }}
        />
      )}
      {discardTarget && (
        <ConfirmDialog
          title="Discard changes"
          description="Restores this file to its committed version. Uncommitted edits to it are lost."
          target={discardTarget}
          confirmLabel="Discard changes"
          busy={discardBusy}
          error={discardError}
          onConfirm={() => discardFile(discardTarget)}
          onCancel={() => {
            setDiscardTarget(null);
            setDiscardError(null);
          }}
        />
      )}
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
                  <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); startSignIn(); }} className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90">Continue</button>
                </div>
                {status && <div className="mt-3 text-xs opacity-70">{status}</div>}
              </>
            ) : (
              <>
                <h2 className="text-lg font-medium">Authorize this app</h2>
                <p className="mt-2 text-sm opacity-80">Enter this code in your browser</p>
                <div className="mt-4 flex items-center justify-center rounded bg-neutral-100 py-3 text-2xl font-mono tracking-widest dark:bg-neutral-800">{userCode}</div>
                <div className="mt-4 flex flex-col gap-2">
                  <button type="button" onClick={(e) => { e.preventDefault(); e.stopPropagation(); verificationUri && open(verificationUri); }} className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90">Open in browser</button>
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
      {showOpenMenu && (
        <>
          <div
            className="fixed inset-0 z-40"
            aria-hidden="true"
            onClick={() => setShowOpenMenu(false)}
          />
          <div
            role="menu"
            aria-label="Repository actions"
            className="fixed bottom-20 right-4 z-50 flex w-48 flex-col gap-0.5 rounded-lg border border-neutral-200 bg-white p-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
          >
            <button
              type="button"
              role="menuitem"
              className="rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              onClick={() => {
                setShowOpenMenu(false);
                openRepo();
              }}
            >
              Open repository…
            </button>
            <button
              type="button"
              role="menuitem"
              className="rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              onClick={() => {
                setShowOpenMenu(false);
                setShowNewRepo(true);
              }}
            >
              New repository…
            </button>
            <button
              type="button"
              role="menuitem"
              className="rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              onClick={() => {
                setShowOpenMenu(false);
                setShowCloneRepo(true);
              }}
            >
              Clone repository…
            </button>
          </div>
        </>
      )}
      <button
        type="button"
        className="fixed bottom-4 right-4 w-12 h-12 bg-gray-700 text-white rounded-full text-2xl shadow-lg z-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
        title="Repository actions"
        aria-label="Repository actions"
        aria-haspopup="menu"
        aria-expanded={showOpenMenu}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setShowOpenMenu((open) => !open);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            setShowOpenMenu(false);
          }
        }}
      >
        +
      </button>
    </div>
  );
}

export default App;
