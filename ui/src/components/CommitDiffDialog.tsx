import { Suspense, lazy, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfirmDialog from "./ConfirmDialog";
import { useDialogFocus } from "../hooks/useDialogFocus";

// Monaco is heavy; load it only when a commit file is opened.
const DiffViewer = lazy(() => import("./DiffViewer"));

type CommitFile = {
  path: string;
  status: string;
};

type CommitDetail = {
  id: string;
  message: string;
  author: string;
  time: number;
  files: CommitFile[];
};

type CommitDiffDialogProps = {
  repoPath: string;
  oid: string;
  dark: boolean;
  tabSize: number;
  sideBySide: boolean;
  ignoreWhitespace: boolean;
  wrap: boolean;
  fontSize: number;
  onClose: () => void;
  /** Called after the repository changed, so the parent can refresh. */
  onApplied: () => void;
};

const STATUS_LABEL: Record<string, string> = {
  added: "added",
  deleted: "deleted",
  modified: "modified",
  renamed: "renamed",
  copied: "copied",
  typechange: "type changed",
  unreadable: "unreadable",
};

/** The three reset modes, with what each does to the index and working tree. */
const RESET_MODES = [
  {
    id: "soft",
    label: "Soft",
    hint: "Move the branch only. The changes stay staged.",
  },
  {
    id: "mixed",
    label: "Mixed",
    hint: "Move the branch and unstage the changes, keeping the files.",
  },
  {
    id: "hard",
    label: "Hard",
    hint: "Move the branch and discard the changes. This cannot be undone.",
  },
] as const;

type ResetMode = (typeof RESET_MODES)[number]["id"];

type ResetDialogProps = {
  oid: string;
  mode: ResetMode;
  onMode: (mode: ResetMode) => void;
  busy: boolean;
  error: string | null;
  onCancel: () => void;
  onConfirm: () => void;
};

/** Sub-dialog that owns its own focus trap while it is open. */
function ResetDialog({
  oid,
  mode,
  onMode,
  busy,
  error,
  onCancel,
  onConfirm,
}: ResetDialogProps) {
  const dialogRef = useDialogFocus<HTMLDivElement>({
    onDismiss: onCancel,
    dismissDisabled: busy,
  });

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/40 p-4">
      <div
        ref={dialogRef}
        tabIndex={-1}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="reset-title"
        aria-describedby="reset-description"
        className="w-full max-w-md rounded-lg border border-neutral-200 bg-white p-4 shadow-xl focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
      >
        <h2 id="reset-title" className="text-sm font-semibold">
          Reset branch to this commit
        </h2>
        <p id="reset-description" className="mt-1 text-xs opacity-70">
          Moves the current branch to{" "}
          <span className="font-mono">{oid.slice(0, 8)}</span>, removing the
          commits after it from the branch. Choose what happens to the changes.
        </p>
        <fieldset className="mt-3 flex flex-col gap-2">
          <legend className="sr-only">Reset mode</legend>
          {RESET_MODES.map((option) => (
            <label
              key={option.id}
              className="flex cursor-pointer items-start gap-2 rounded px-1 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800"
            >
              <input
                type="radio"
                name="reset-mode"
                value={option.id}
                checked={mode === option.id}
                onChange={() => onMode(option.id)}
                disabled={busy}
                className="mt-0.5"
              />
              <span className="min-w-0">
                <span className="font-medium">{option.label}</span>
                <span className="block text-xs opacity-70">{option.hint}</span>
              </span>
            </label>
          ))}
        </fieldset>
        {error && (
          <p role="alert" className="mt-2 text-xs text-red-700 dark:text-red-400">
            {error}
          </p>
        )}
        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            disabled={busy}
            className="rounded px-3 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={busy}
            className="rounded bg-red-600 px-3 py-1 text-sm text-white hover:bg-red-500 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            {busy ? "Resetting…" : "Reset branch"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default function CommitDiffDialog({
  repoPath,
  oid,
  dark,
  tabSize,
  sideBySide,
  ignoreWhitespace,
  wrap,
  fontSize,
  onClose,
  onApplied,
}: CommitDiffDialogProps) {
  const [detail, setDetail] = useState<CommitDetail | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [contents, setContents] = useState<{ original: string; modified: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loadingFile, setLoadingFile] = useState(false);
  const [confirm, setConfirm] = useState<"cherry-pick" | "revert" | null>(null);
  const [actionBusy, setActionBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [resetOpen, setResetOpen] = useState(false);
  const [resetMode, setResetMode] = useState<"soft" | "mixed" | "hard">("mixed");
  const [resetBusy, setResetBusy] = useState(false);
  const [resetError, setResetError] = useState<string | null>(null);
  const closeRef = useRef<HTMLButtonElement | null>(null);
  const dialogRef = useDialogFocus<HTMLDivElement>({
    onDismiss: () => {
      // A sub-dialog handles its own Escape first.
      if (!confirm && !resetOpen) onClose();
    },
    initialFocus: closeRef,
    focusTrap: !confirm && !resetOpen,
  });

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await invoke<CommitDetail>("repo_commit_detail", { path: repoPath, oid });
        if (cancelled) return;
        setDetail(res);
        const first = res.files[0]?.path ?? null;
        setSelected(first);
      } catch {
        if (!cancelled) setError("Could not load that commit.");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [repoPath, oid]);

  useEffect(() => {
    if (!selected) {
      setContents(null);
      return;
    }
    let cancelled = false;
    setLoadingFile(true);
    setError(null);
    (async () => {
      try {
        const res = await invoke<{ original: string; modified: string }>("repo_commit_file", {
          path: repoPath,
          oid,
          file: selected,
        });
        if (!cancelled) setContents(res);
      } catch {
        if (!cancelled) {
          setContents(null);
          setError("Could not load that file's changes.");
        }
      } finally {
        if (!cancelled) setLoadingFile(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [repoPath, oid, selected]);

  async function apply(kind: "cherry-pick" | "revert") {
    setActionBusy(true);
    setActionError(null);
    try {
      await invoke(kind === "cherry-pick" ? "repo_cherry_pick" : "repo_revert", {
        path: repoPath,
        oid,
      });
      setConfirm(null);
      // A conflict is not an error: the operation stays in progress so the
      // operation panel can resolve it.
      let pending = false;
      try {
        const state = await invoke<string>("repo_operation_state", {
          path: repoPath,
        });
        pending = state !== "none";
      } catch {
        pending = false;
      }
      setActionMessage(
        pending
          ? `The ${kind} stopped with conflicts. Resolve them in the operation panel.`
          : kind === "cherry-pick"
            ? "Cherry-picked onto the current branch."
            : "Reverted on the current branch.",
      );
      onApplied();
    } catch (e) {
      setActionError(
        typeof e === "string"
          ? e
          : kind === "cherry-pick"
            ? "Could not cherry-pick that commit."
            : "Could not revert that commit.",
      );
    } finally {
      setActionBusy(false);
    }
  }

  async function resetHere() {
    setResetBusy(true);
    setResetError(null);
    try {
      await invoke("repo_reset", {
        path: repoPath,
        mode: resetMode,
        target: oid,
      });
      setResetOpen(false);
      setActionMessage(
        resetMode === "hard"
          ? "Branch reset to this commit. Working tree changes were discarded."
          : resetMode === "mixed"
            ? "Branch reset to this commit. Its changes are in the working tree."
            : "Branch reset to this commit. Its changes are staged.",
      );
      onApplied();
    } catch (e) {
      setResetError(
        typeof e === "string" ? e : "Could not reset the branch.",
      );
    } finally {
      setResetBusy(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        ref={dialogRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby="commit-diff-title"
        className="flex h-[80vh] w-full max-w-5xl flex-col rounded-lg border border-neutral-200 bg-white shadow-xl focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
      >
        <div className="flex items-start justify-between gap-3 border-b border-neutral-200 p-3 dark:border-neutral-800">
          <div className="min-w-0">
            <h2 id="commit-diff-title" className="truncate text-sm font-semibold">
              {detail ? detail.message.split("\n")[0] : "Commit"}
            </h2>
            <p className="mt-1 truncate text-xs opacity-60">
              {detail
                ? `${detail.id.slice(0, 8)} · ${detail.author}`
                : oid.slice(0, 8)}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <button
              type="button"
              disabled={actionBusy}
              onClick={() => {
                setActionError(null);
                setConfirm("cherry-pick");
              }}
              className="rounded px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Cherry-pick
            </button>
            <button
              type="button"
              disabled={actionBusy}
              onClick={() => {
                setActionError(null);
                setConfirm("revert");
              }}
              className="rounded px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Revert
            </button>
            <button
              type="button"
              disabled={actionBusy}
              onClick={() => {
                setResetError(null);
                setResetMode("mixed");
                setResetOpen(true);
              }}
              className="rounded px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Reset branch…
            </button>
            <button
              ref={closeRef}
              type="button"
              onClick={onClose}
              aria-label="Close commit"
              className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              ✕
            </button>
          </div>
        </div>

        <div className="flex min-h-0 flex-1">
          <div className="w-64 shrink-0 overflow-y-auto border-r border-neutral-200 p-2 dark:border-neutral-800">
            <h3 className="px-2 pb-1 text-xs font-semibold opacity-70">Changed files</h3>
            {detail && detail.files.length === 0 ? (
              <p className="px-2 text-xs opacity-60">No changed files.</p>
            ) : (
              <ul className="space-y-0.5">
                {detail?.files.map((file) => (
                  <li key={file.path}>
                    <button
                      type="button"
                      onClick={() => setSelected(file.path)}
                      aria-current={selected === file.path ? "true" : undefined}
                      className={`flex w-full items-center justify-between gap-2 rounded px-2 py-1 text-left text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
                        selected === file.path
                          ? "bg-neutral-100 font-medium dark:bg-neutral-800"
                          : "hover:bg-neutral-100 dark:hover:bg-neutral-800"
                      }`}
                    >
                      <span className="min-w-0 truncate" title={file.path}>
                        {file.path}
                      </span>
                      <span className="shrink-0 text-xs opacity-60">
                        {STATUS_LABEL[file.status] ?? file.status}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="min-h-0 flex-1">
            {loadingFile ? (
              <div className="p-4 text-sm opacity-60">Loading diff…</div>
            ) : contents && selected ? (
              <Suspense fallback={<div className="p-4 text-sm opacity-60">Loading editor…</div>}>
                <DiffViewer
                  original={contents.original}
                  modified={contents.modified}
                  fileName={selected}
                  tabSize={tabSize}
                  sideBySide={sideBySide}
                  ignoreWhitespace={ignoreWhitespace}
                  wrap={wrap}
                  fontSize={fontSize}
                  dark={dark}
                />
              </Suspense>
            ) : (
              <div className="p-4 text-sm opacity-60">
                {detail && detail.files.length > 0
                  ? "Select a file to view its changes."
                  : "No changes to show."}
              </div>
            )}
          </div>
        </div>

        {error && (
          <p role="alert" className="border-t border-neutral-200 p-2 text-xs text-red-700 dark:border-neutral-800 dark:text-red-400">
            {error}
          </p>
        )}
        {actionMessage && (
          <p role="status" className="border-t border-neutral-200 p-2 text-xs opacity-70 dark:border-neutral-800">
            {actionMessage}
          </p>
        )}
      </div>

      {resetOpen && (
        <ResetDialog
          oid={oid}
          mode={resetMode}
          onMode={setResetMode}
          busy={resetBusy}
          error={resetError}
          onCancel={() => setResetOpen(false)}
          onConfirm={resetHere}
        />
      )}

      {confirm && (
        <ConfirmDialog
          title={confirm === "cherry-pick" ? "Cherry-pick commit" : "Revert commit"}
          description={
            confirm === "cherry-pick"
              ? "Applies this commit's changes on top of the current branch as a new commit. If the changes conflict, the cherry-pick stays in progress so you can resolve them."
              : "Creates a new commit that undoes this commit's changes on the current branch. If the changes conflict, the revert stays in progress so you can resolve them."
          }
          target={detail ? detail.message.split("\n")[0] : oid.slice(0, 8)}
          confirmLabel={confirm === "cherry-pick" ? "Cherry-pick" : "Revert"}
          busy={actionBusy}
          error={actionError}
          onConfirm={() => apply(confirm)}
          onCancel={() => {
            setConfirm(null);
            setActionError(null);
          }}
        />
      )}
    </div>
  );
}
