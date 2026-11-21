import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useDialogFocus } from "../hooks/useDialogFocus";

type Action = "pick" | "reword" | "squash" | "fixup" | "drop";

type PlanCommit = {
  id: string;
  summary: string;
  author: string;
  time: number;
  message: string;
};

type Row = PlanCommit & { action: Action; message: string };

type InteractiveRebaseDialogProps = {
  repoPath: string;
  /** Branch the current branch is replayed on top of. */
  onto: string;
  currentBranch?: string;
  /** Called after the rebase ran (completed or stopped at a conflict). */
  onApplied: () => void;
  onClose: () => void;
};

const ACTIONS: { value: Action; label: string }[] = [
  { value: "pick", label: "Pick" },
  { value: "reword", label: "Reword" },
  { value: "squash", label: "Squash" },
  { value: "fixup", label: "Fix up" },
  { value: "drop", label: "Drop" },
];

/** Mirrors the backend rules so obvious mistakes are caught before invoking. */
function validate(rows: Row[]): string | null {
  if (rows.length === 0) {
    return "There are no commits to rebase.";
  }
  let keptSeen = false;
  for (const row of rows) {
    if (row.action !== "drop") {
      if (!keptSeen) {
        if (row.action === "squash" || row.action === "fixup") {
          return "The first kept commit cannot be squashed or fixed up.";
        }
        keptSeen = true;
      }
    }
    if (row.action === "reword" && !row.message.trim()) {
      return "Every reworded commit needs a message.";
    }
  }
  return null;
}

/**
 * Reorders, squashes, rewrites and drops the commits of the current branch
 * before they are replayed onto another branch.
 */
export default function InteractiveRebaseDialog({
  repoPath,
  onto,
  currentBranch,
  onApplied,
  onClose,
}: InteractiveRebaseDialogProps) {
  const [rows, setRows] = useState<Row[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const dialogRef = useDialogFocus<HTMLDivElement>({
    onDismiss: onClose,
    dismissDisabled: busy,
    initialFocus: cancelRef,
  });

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const commits = await invoke<PlanCommit[]>("repo_rebase_plan", {
          path: repoPath,
          onto,
        });
        if (cancelled) return;
        setRows(
          (Array.isArray(commits) ? commits : []).map((commit) => ({
            ...commit,
            action: "pick" as Action,
            message: commit.message,
          })),
        );
      } catch (e) {
        if (!cancelled) {
          setError(typeof e === "string" ? e : "Could not read the commits to rebase.");
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, [repoPath, onto]);

  function move(index: number, delta: number) {
    setRows((current) => {
      const target = index + delta;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  }

  function setAction(index: number, action: Action) {
    setRows((current) =>
      current.map((row, i) => (i === index ? { ...row, action } : row)),
    );
  }

  function setMessage(index: number, message: string) {
    setRows((current) =>
      current.map((row, i) => (i === index ? { ...row, message } : row)),
    );
  }

  async function start() {
    const problem = validate(rows);
    if (problem) {
      setError(problem);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await invoke<string>("repo_interactive_rebase", {
        path: repoPath,
        onto,
        steps: rows.map((row) => ({
          oid: row.id,
          action: row.action,
          message: row.action === "reword" ? row.message : null,
        })),
      });
      // A conflict leaves the rebase in progress, which the operation panel
      // shows; either way the repository changed.
      onApplied();
      onClose();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not rebase those commits.");
    } finally {
      setBusy(false);
    }
  }

  const keptCount = rows.filter((row) => row.action !== "drop").length;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        ref={dialogRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby="interactive-rebase-title"
        aria-describedby="interactive-rebase-description"
        className="flex max-h-[85vh] w-full max-w-2xl flex-col rounded-lg border border-neutral-200 bg-white p-4 shadow-xl focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
      >
        <h2 id="interactive-rebase-title" className="text-sm font-semibold">
          Interactive rebase onto {onto}
        </h2>
        <p id="interactive-rebase-description" className="mt-1 text-xs opacity-70">
          Commits are replayed in this order on top of {onto}. Squash and fix up
          fold a commit into the one above it; drop leaves it out. Rewriting
          gives every changed commit a new id.
        </p>

        {loading ? (
          <p className="mt-4 text-sm opacity-60">Reading commits…</p>
        ) : rows.length === 0 ? (
          <p className="mt-4 text-sm opacity-60">
            There are no commits on {currentBranch ?? "this branch"} that are
            missing from {onto}.
          </p>
        ) : (
          <ul className="mt-3 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1">
            {rows.map((row, index) => (
              <li
                key={row.id}
                className="rounded border border-neutral-200 p-2 dark:border-neutral-700"
              >
                <div className="flex items-start gap-2">
                  <div className="flex shrink-0 flex-col gap-0.5">
                    <button
                      type="button"
                      onClick={() => move(index, -1)}
                      disabled={index === 0 || busy}
                      aria-label={`Move ${row.summary} up`}
                      className="rounded px-1 text-xs hover:bg-neutral-100 disabled:opacity-30 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      onClick={() => move(index, 1)}
                      disabled={index === rows.length - 1 || busy}
                      aria-label={`Move ${row.summary} down`}
                      className="rounded px-1 text-xs hover:bg-neutral-100 disabled:opacity-30 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                    >
                      ↓
                    </button>
                  </div>

                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <select
                        value={row.action}
                        onChange={(event) =>
                          setAction(index, event.target.value as Action)
                        }
                        disabled={busy}
                        aria-label={`Action for ${row.summary}`}
                        className="shrink-0 rounded border border-neutral-300 bg-white px-1 py-0.5 text-xs dark:border-neutral-700 dark:bg-neutral-900"
                      >
                        {ACTIONS.map((action) => (
                          <option key={action.value} value={action.value}>
                            {action.label}
                          </option>
                        ))}
                      </select>
                      <span className="min-w-0 truncate text-sm">
                        {row.summary}
                      </span>
                    </div>
                    <p className="mt-0.5 truncate font-mono text-xs opacity-60">
                      {row.id.slice(0, 7)} · {row.author}
                    </p>

                    {row.action === "reword" && (
                      <textarea
                        value={row.message}
                        onChange={(event) => setMessage(index, event.target.value)}
                        disabled={busy}
                        spellCheck={false}
                        aria-label={`New message for ${row.summary}`}
                        className="mt-1 h-20 w-full rounded border border-neutral-300 bg-white p-2 font-mono text-xs dark:border-neutral-700 dark:bg-neutral-900"
                      />
                    )}
                    {(row.action === "squash" || row.action === "fixup") && (
                      <p className="mt-1 text-xs opacity-60">
                        Folds into the commit above.
                      </p>
                    )}
                    {row.action === "drop" && (
                      <p className="mt-1 text-xs opacity-60">
                        This commit is left out.
                      </p>
                    )}
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}

        {!loading && rows.length > 0 && keptCount === 0 && (
          <p className="mt-2 text-xs text-amber-700 dark:text-amber-400">
            Every commit is dropped. The branch ends at {onto}; the commits stay
            reachable from the reflog.
          </p>
        )}

        {error && (
          <p role="alert" className="mt-2 text-xs text-red-700 dark:text-red-400">
            {error}
          </p>
        )}

        <div className="mt-3 flex items-center justify-between gap-2">
          <p className="min-w-0 truncate text-xs opacity-60">
            {currentBranch ?? "Current branch"} → {onto}
          </p>
          <div className="flex shrink-0 gap-2">
            <button
              ref={cancelRef}
              type="button"
              onClick={onClose}
              disabled={busy}
              className="rounded px-3 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={start}
              disabled={busy || loading || rows.length === 0}
              className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              {busy ? "Rebasing…" : "Start rebase"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
