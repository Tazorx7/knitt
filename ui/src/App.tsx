import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-shell";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

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

function App() {
  const [secretOk, setSecretOk] = useState<boolean | null>(null);
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

  useEffect(() => {
    invoke<boolean>("auth_check_secret_service").then(setSecretOk).catch(() => setSecretOk(false));
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
    const unlisten4 = listen("auth:error", (e: any) => setStatus(`Error: ${e.payload.error}`));
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
      setStatus(`Error: ${e}`);
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
    await invoke("auth_cancel_device_flow");
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
        setStatus(`Repo error: ${e}`);
      }
    }
  }

  async function refreshStatus(path: string) {
    try {
      const status: FileStatus[] = await invoke("repo_status", { path });
      setFiles(status);
    } catch (e: any) {
      setStatus(`Status error: ${e}`);
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
      setStatus(`Commit error: ${e}`);
    }
  }

  async function push() {
    if (!repo) return;
    try {
      await invoke("repo_push", { path: repo.path });
      setStatus("Pushed");
    } catch (e: any) {
      setStatus(`Push error: ${e}`);
    }
  }

  const active = accounts.find(a => a.id === activeId);

  return (
    <div className="min-h-screen bg-neutral-50 text-neutral-900 dark:bg-neutral-950 dark:text-neutral-50">
      <header className="flex items-center justify-between border-b border-neutral-200 px-4 py-2 dark:border-neutral-800">
        <div className="flex items-center gap-4">
          <div className="font-medium">Native Git Client</div>
          <button onClick={openRepo} className="rounded px-2 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Open repository</button>
          {repo && (
            <>
              <div className="text-sm opacity-80">{repo.name} {repo.branch ? `• ${repo.branch}` : ""}</div>
              <button onClick={push} className="rounded px-2 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Push</button>
            </>
          )}
        </div>
        <div className="flex items-center gap-2">
          {active ? (
            <div className="flex items-center gap-2 text-sm">
              <img src={active.avatar_url} className="h-6 w-6 rounded-full" />
              <span>{active.login}</span>
              <button onClick={() => signOut(active.id)} className="rounded px-2 py-1 hover:bg-neutral-100 dark:hover:bg-neutral-800">Sign out</button>
            </div>
          ) : (
            <button onClick={() => setShowSignIn(true)} className="rounded px-2 py-1 hover:bg-neutral-100 dark:hover:bg-neutral-800">Sign in to GitHub</button>
          )}
        </div>
      </header>
      <main className="flex h-[calc(100vh-3rem)]">
        <aside className="w-80 border-r border-neutral-200 p-3 dark:border-neutral-800">
          <div className="mb-2 text-sm font-medium">Changes</div>
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
              />
              <button onClick={commit} className="rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200">Commit</button>
            </div>
          )}
        </aside>
        <section className="flex-1 p-4">
          <div className="text-sm opacity-80">Secret Service: {secretOk === null ? "checking..." : secretOk ? "available" : "unavailable"}</div>
          {status && <div className="mt-2 text-xs opacity-70">{status}</div>}
        </section>
      </main>

      {showSignIn && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/40">
          <div className="w-[420px] rounded-lg border border-neutral-200 bg-white p-6 shadow-lg dark:border-neutral-800 dark:bg-neutral-900">
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
                  <button onClick={cancel} className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Cancel</button>
                  <button onClick={startSignIn} className="rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200">Continue</button>
                </div>
                {status && <div className="mt-3 text-xs opacity-70">{status}</div>}
              </>
            ) : (
              <>
                <h2 className="text-lg font-medium">Authorize this app</h2>
                <p className="mt-2 text-sm opacity-80">Enter this code in your browser</p>
                <div className="mt-4 flex items-center justify-center rounded bg-neutral-100 py-3 text-2xl font-mono tracking-widest dark:bg-neutral-800">{userCode}</div>
                <div className="mt-4 flex flex-col gap-2">
                  <button onClick={() => verificationUri && open(verificationUri)} className="rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200">Open in browser</button>
                  <button onClick={() => verificationUri && navigator.clipboard.writeText(verificationUri)} className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Copy verification URL</button>
                </div>
                <div className="mt-4 text-xs opacity-70">{status}</div>
                <div className="mt-4 flex justify-end">
                  <button onClick={cancel} className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800">Cancel</button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export default App;

