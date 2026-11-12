import { Suspense, lazy, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
}: CommitDiffDialogProps) {
  const [detail, setDetail] = useState<CommitDetail | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [contents, setContents] = useState<{ original: string; modified: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loadingFile, setLoadingFile] = useState(false);
  const closeRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="commit-diff-title"
        className="flex h-[80vh] w-full max-w-5xl flex-col rounded-lg border border-neutral-200 bg-white shadow-xl dark:border-neutral-700 dark:bg-neutral-900"
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
          <button
            ref={closeRef}
            type="button"
            onClick={onClose}
            aria-label="Close commit"
            className="shrink-0 rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            ✕
          </button>
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
          <p role="status" className="border-t border-neutral-200 p-2 text-xs text-red-700 dark:border-neutral-800 dark:text-red-400">
            {error}
          </p>
        )}
      </div>
    </div>
  );
}
