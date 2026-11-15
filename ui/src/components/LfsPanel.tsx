import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type LfsStatus = { installed: boolean; version: string | null };
type TrackedPattern = { pattern: string; source: string };

type LfsPanelProps = {
  repoPath: string;
  /** Called after `.gitattributes` or the object store changed. */
  onChanged?: () => void;
};

const button =
  "rounded px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]";
const input =
  "min-w-0 rounded border border-neutral-300 bg-white px-2 py-1 text-xs outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] dark:border-neutral-700 dark:bg-neutral-900";

/** Tracks large-file patterns and moves LFS objects through the git-lfs CLI. */
export default function LfsPanel({ repoPath, onChanged }: LfsPanelProps) {
  const [status, setStatus] = useState<LfsStatus | null>(null);
  const [tracked, setTracked] = useState<TrackedPattern[]>([]);
  const [pattern, setPattern] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setStatus(null);
    setTracked([]);
    setError(null);
    setMessage(null);
    invoke<LfsStatus>("repo_lfs_status")
      .then(async (next) => {
        if (!active) return;
        setStatus(next);
        if (!next.installed) return;
        try {
          const patterns = await invoke<TrackedPattern[]>("repo_lfs_tracked", {
            path: repoPath,
          });
          if (active) setTracked(Array.isArray(patterns) ? patterns : []);
        } catch (e) {
          if (active) {
            setError(typeof e === "string" ? e : "Could not read the LFS patterns.");
          }
        }
      })
      .catch(() => {
        if (active) {
          setStatus({ installed: false, version: null });
        }
      });
    return () => {
      active = false;
    };
  }, [repoPath]);

  async function loadPatterns() {
    const patterns = await invoke<TrackedPattern[]>("repo_lfs_tracked", {
      path: repoPath,
    });
    setTracked(Array.isArray(patterns) ? patterns : []);
  }

  async function track(event: React.FormEvent) {
    event.preventDefault();
    if (!pattern.trim() || busy) return;
    setBusy("track");
    setError(null);
    setMessage(null);
    try {
      await invoke("repo_lfs_track", { path: repoPath, pattern });
      setPattern("");
      await loadPatterns();
      setMessage("Added the pattern. Commit the .gitattributes change to share it.");
      onChanged?.();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not track that pattern.");
    } finally {
      setBusy(null);
    }
  }

  async function untrack(pattern: string) {
    setBusy(`untrack:${pattern}`);
    setError(null);
    setMessage(null);
    try {
      await invoke("repo_lfs_untrack", { path: repoPath, pattern });
      await loadPatterns();
      setMessage(`Stopped tracking ${pattern}.`);
      onChanged?.();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not untrack that pattern.");
    } finally {
      setBusy(null);
    }
  }

  async function transfer(direction: "pull" | "push") {
    setBusy(direction);
    setError(null);
    setMessage(null);
    try {
      await invoke(direction === "pull" ? "repo_lfs_pull" : "repo_lfs_push", {
        path: repoPath,
      });
      setMessage(
        direction === "pull"
          ? "Downloaded the LFS objects."
          : "Uploaded the LFS objects.",
      );
    } catch (e) {
      setError(
        typeof e === "string"
          ? e
          : direction === "pull"
            ? "Could not download the LFS objects."
            : "Could not upload the LFS objects.",
      );
    } finally {
      setBusy(null);
    }
  }

  if (!status) {
    return <p className="text-xs opacity-60">Checking Git LFS…</p>;
  }

  if (!status.installed) {
    return (
      <p className="text-xs opacity-70">
        Git LFS is not installed. Install the <code>git-lfs</code> package to
        track large files with this repository; Knitt only manages the patterns
        and transfers through that command.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-2 text-xs">
      <p className="opacity-60">{status.version ?? "Git LFS"}</p>

      <form onSubmit={track} className="flex items-center gap-1">
        <input
          value={pattern}
          onChange={(event) => setPattern(event.target.value)}
          placeholder="*.psd"
          aria-label="LFS pattern"
          spellCheck={false}
          className={input}
        />
        <button
          type="submit"
          disabled={busy !== null || !pattern.trim()}
          className={button}
        >
          {busy === "track" ? "Tracking…" : "Track"}
        </button>
      </form>

      {tracked.length === 0 ? (
        <p className="opacity-60">No patterns tracked.</p>
      ) : (
        <ul className="flex flex-col gap-0.5">
          {tracked.map((entry) => (
            <li key={`${entry.source}:${entry.pattern}`} className="flex items-center gap-1">
              <span className="min-w-0 flex-1 truncate font-mono" title={entry.source}>
                {entry.pattern}
              </span>
              <button
                type="button"
                onClick={() => untrack(entry.pattern)}
                disabled={busy !== null}
                aria-label={`Stop tracking ${entry.pattern}`}
                className={button}
              >
                {busy === `untrack:${entry.pattern}` ? "Removing…" : "Remove"}
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="flex flex-wrap items-center gap-1">
        <button
          type="button"
          onClick={() => transfer("pull")}
          disabled={busy !== null}
          title="Download objects a clone is missing"
          className={button}
        >
          {busy === "pull" ? "Downloading…" : "Download objects"}
        </button>
        <button
          type="button"
          onClick={() => transfer("push")}
          disabled={busy !== null}
          title="Upload objects before pushing the branch; Knitt's own push does not run Git hooks"
          className={button}
        >
          {busy === "push" ? "Uploading…" : "Upload objects"}
        </button>
      </div>

      {message && <p className="text-green-700 dark:text-green-400">{message}</p>}
      {error && (
        <p role="alert" className="text-red-700 dark:text-red-400">
          {error}
        </p>
      )}
      <p className="opacity-60">
        Checkouts and commits use the filters git-lfs configured for this
        repository.
      </p>
    </div>
  );
}
