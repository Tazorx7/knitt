import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Hunk = {
  header: string;
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  added: number;
  removed: number;
};

type HunkBarProps = {
  repoPath: string;
  file: string;
  /** Bumped by the parent whenever the repository is re-read. */
  refreshSignal: number;
  onChanged: () => void;
};

function shortHeader(hunk: Hunk): string {
  const location =
    hunk.old_lines === 0
      ? `after line ${hunk.old_start}`
      : `line ${hunk.old_start}`;
  return location;
}

/** Per-hunk Stage and Unstage buttons for the file being viewed. */
export default function HunkBar({
  repoPath,
  file,
  refreshSignal,
  onChanged,
}: HunkBarProps) {
  const [unstaged, setUnstaged] = useState<Hunk[]>([]);
  const [staged, setStaged] = useState<Hunk[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [nextUnstaged, nextStaged] = await Promise.all([
          invoke<Hunk[]>("repo_list_hunks", {
            path: repoPath,
            file,
            staged: false,
          }),
          invoke<Hunk[]>("repo_list_hunks", {
            path: repoPath,
            file,
            staged: true,
          }),
        ]);
        if (cancelled) return;
        setUnstaged(Array.isArray(nextUnstaged) ? nextUnstaged : []);
        setStaged(Array.isArray(nextStaged) ? nextStaged : []);
        setError(null);
      } catch {
        if (!cancelled) {
          setUnstaged([]);
          setStaged([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [repoPath, file, refreshSignal]);

  async function apply(hunk: Hunk, stage: boolean) {
    setBusy(`${stage ? "stage" : "unstage"}:${hunk.header}`);
    setError(null);
    try {
      await invoke(stage ? "repo_stage_hunk" : "repo_unstage_hunk", {
        path: repoPath,
        file,
        hunkHeader: hunk.header,
      });
      onChanged();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not change that hunk.");
    } finally {
      setBusy(null);
    }
  }

  if (unstaged.length === 0 && staged.length === 0 && !error) {
    return null;
  }

  const groups = [
    { label: "Unstaged hunks", hunks: unstaged, stage: true },
    { label: "Staged hunks", hunks: staged, stage: false },
  ].filter((group) => group.hunks.length > 0);

  return (
    <div className="border-b border-neutral-200 px-4 py-2 dark:border-neutral-800">
      {groups.map((group) => (
        <div key={group.label} className="flex flex-wrap items-center gap-1 py-0.5">
          <span className="mr-1 text-xs opacity-60">{group.label}</span>
          {group.hunks.map((hunk) => {
            const key = `${group.stage ? "stage" : "unstage"}:${hunk.header}`;
            return (
              <span
                key={hunk.header}
                className="flex items-center gap-1 rounded border border-neutral-200 px-1.5 py-0.5 text-xs dark:border-neutral-700"
              >
                <span className="font-mono opacity-70">{shortHeader(hunk)}</span>
                <span className="text-green-700 dark:text-green-400">
                  +{hunk.added}
                </span>
                <span className="text-red-700 dark:text-red-400">
                  −{hunk.removed}
                </span>
                <button
                  type="button"
                  onClick={() => apply(hunk, group.stage)}
                  disabled={busy !== null}
                  title={
                    group.stage
                      ? `Stage ${hunk.header}`
                      : `Unstage ${hunk.header}`
                  }
                  className="rounded px-1 hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                >
                  {busy === key
                    ? "…"
                    : group.stage
                      ? "Stage hunk"
                      : "Unstage hunk"}
                </button>
              </span>
            );
          })}
        </div>
      ))}
      {error && (
        <p role="alert" className="pt-1 text-xs text-red-700 dark:text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}
