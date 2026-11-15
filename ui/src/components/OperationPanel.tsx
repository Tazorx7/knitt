import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfirmDialog from "./ConfirmDialog";

type OperationState =
  | "none"
  | "merge"
  | "rebase"
  | "cherry-pick"
  | "revert"
  | "other";

type OperationPanelProps = {
  repoPath: string;
  /** Bumped by the parent whenever the repository is re-read, so the panel
   *  reloads without polling. */
  refreshSignal: number;
  /** Called after an action changed the repository. */
  onChanged: () => void;
};

/** Verb form, used in sentences such as "Continue to finish the merge". */
function describe(state: OperationState): string {
  switch (state) {
    case "merge":
      return "merge";
    case "rebase":
      return "rebase";
    case "cherry-pick":
      return "cherry-pick";
    case "revert":
      return "revert";
    default:
      return "operation";
  }
}

/** Title form, used for the banner heading. */
function heading(state: OperationState): string {
  switch (state) {
    case "merge":
      return "Merging";
    case "rebase":
      return "Rebasing";
    case "cherry-pick":
      return "Cherry-picking";
    case "revert":
      return "Reverting";
    default:
      return "Operation in progress";
  }
}

/**
 * Shows the in-progress merge, rebase, cherry-pick or revert, lets each
 * conflicted file be edited and staged, and finishes or abandons it.
 */
export default function OperationPanel({
  repoPath,
  refreshSignal,
  onChanged,
}: OperationPanelProps) {
  const [state, setState] = useState<OperationState>("none");
  const [conflicts, setConflicts] = useState<string[]>([]);
  const [openFile, setOpenFile] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [loadedContent, setLoadedContent] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [aborting, setAborting] = useState(false);

  async function reload() {
    try {
      const next = await invoke<OperationState>("repo_operation_state", {
        path: repoPath,
      });
      const files = await invoke<string[]>("repo_list_conflicts", {
        path: repoPath,
      });
      setState(next);
      setConflicts(Array.isArray(files) ? files : []);
      // Drop the editor when that file is no longer conflicted.
      setOpenFile((cur) =>
        cur && Array.isArray(files) && files.includes(cur) ? cur : null,
      );
    } catch {
      setState("none");
      setConflicts([]);
    }
  }

  useEffect(() => {
    setOpenFile(null);
    setContent("");
    setError(null);
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoPath, refreshSignal]);

  async function openConflict(file: string) {
    setError(null);
    setBusy(`open:${file}`);
    try {
      const text = await invoke<string>("repo_get_conflict", {
        path: repoPath,
        file,
      });
      setOpenFile(file);
      setContent(text);
      setLoadedContent(text);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not open that file.");
    } finally {
      setBusy(null);
    }
  }

  /** Opens a conflicted file in the editor configured in Settings. */
  async function openInEditor(file: string) {
    setBusy(`edit:${file}`);
    setError(null);
    try {
      await invoke("repo_open_in_editor", { path: repoPath, file });
    } catch (e) {
      setError(
        typeof e === "string" ? e : "Could not open that file in the editor.",
      );
    } finally {
      setBusy(null);
    }
  }

  /** Re-reads the conflicted file after it was edited outside Knitt. The
   *  content stays dirty so Save resolution stages the external edits. */
  async function reloadFile() {
    if (!openFile) return;
    setBusy("reload");
    setError(null);
    try {
      const text = await invoke<string>("repo_get_conflict", {
        path: repoPath,
        file: openFile,
      });
      setContent(text);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not reload that file.");
    } finally {
      setBusy(null);
    }
  }

  async function saveResolution() {
    if (!openFile) return;
    setBusy("save");
    setError(null);
    try {
      await invoke("repo_resolve_conflict", {
        path: repoPath,
        file: openFile,
        content,
      });
      setLoadedContent(content);
      await reload();
      onChanged();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not save the resolution.");
    } finally {
      setBusy(null);
    }
  }

  async function finish() {
    setBusy("finish");
    setError(null);
    try {
      switch (state) {
        case "merge":
          await invoke("repo_merge_continue", { path: repoPath, message: null });
          break;
        case "rebase":
          await invoke("repo_rebase_continue", { path: repoPath });
          break;
        case "cherry-pick":
          await invoke("repo_cherry_pick_continue", { path: repoPath });
          break;
        case "revert":
          await invoke("repo_revert_continue", { path: repoPath });
          break;
      }
      setOpenFile(null);
      setContent("");
      await reload();
      onChanged();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not finish the operation.");
    } finally {
      setBusy(null);
    }
  }

  async function abort() {
    setBusy("abort");
    setError(null);
    try {
      switch (state) {
        case "merge":
          await invoke("repo_merge_abort", { path: repoPath });
          break;
        case "rebase":
          await invoke("repo_rebase_abort", { path: repoPath });
          break;
        case "cherry-pick":
          await invoke("repo_cherry_pick_abort", { path: repoPath });
          break;
        case "revert":
          await invoke("repo_revert_abort", { path: repoPath });
          break;
      }
      setAborting(false);
      setOpenFile(null);
      setContent("");
      await reload();
      onChanged();
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not abandon the operation.");
    } finally {
      setBusy(null);
    }
  }

  /** States the panel can drive; anything else is shown read-only. */
  const actionable = ["merge", "rebase", "cherry-pick", "revert"].includes(state);

  function finishLabel(): string {
    switch (state) {
      case "merge":
        return "Commit merge";
      case "cherry-pick":
        return "Commit cherry-pick";
      case "revert":
        return "Commit revert";
      default:
        return "Continue";
    }
  }
  const markerLines = useMemo(
    () => content.split("\n").filter((line) => /^(<<<<<<<|=======|>>>>>>>)/.test(line)).length,
    [content],
  );
  const dirty = openFile !== null && content !== loadedContent;

  if (state === "none" && conflicts.length === 0) {
    return null;
  }

  return (
    <div className="mb-3 rounded border border-amber-300 bg-amber-50 p-2 text-sm dark:border-amber-700 dark:bg-amber-950/40">
      <div className="flex items-center justify-between gap-2">
        <strong className="min-w-0 truncate">{heading(state)}</strong>
        {actionable && (
          <div className="flex shrink-0 gap-1">
            <button
              type="button"
              onClick={finish}
              disabled={busy !== null || conflicts.length > 0}
              title={
                conflicts.length > 0
                  ? "Resolve every conflict first"
                  : "Complete the operation"
              }
              className="rounded bg-[var(--accent)] px-2 py-1 text-xs text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              {finishLabel()}
            </button>
            <button
              type="button"
              onClick={() => setAborting(true)}
              disabled={busy !== null}
              className="rounded px-2 py-1 text-xs text-red-700 hover:bg-red-50 disabled:opacity-50 dark:text-red-400 dark:hover:bg-red-950 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Abort
            </button>
          </div>
        )}
      </div>

      {conflicts.length === 0 ? (
        <p className="mt-1 text-xs opacity-70">
          No conflicts left. Continue to finish the {describe(state)}.
        </p>
      ) : (
        <>
          <p className="mt-1 text-xs opacity-70">
            {conflicts.length} conflicted{" "}
            {conflicts.length === 1 ? "file" : "files"}. Edit each one here or in
            your external editor to remove the markers, then save it to stage the
            resolution.
          </p>
          <ul className="mt-2 flex flex-col gap-0.5">
            {conflicts.map((file) => (
              <li key={file} className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => openConflict(file)}
                  aria-current={openFile === file ? "true" : undefined}
                  disabled={busy === `open:${file}`}
                  className={`min-w-0 flex-1 truncate rounded px-2 py-1 text-left text-xs hover:bg-amber-100 disabled:opacity-60 dark:hover:bg-amber-900/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
                    openFile === file ? "bg-amber-100 font-medium dark:bg-amber-900/40" : ""
                  }`}
                >
                  {file}
                </button>
                <button
                  type="button"
                  onClick={() => openInEditor(file)}
                  disabled={busy !== null}
                  aria-label={`Open ${file} in the external editor`}
                  className="shrink-0 rounded px-2 py-1 text-xs hover:bg-amber-100 disabled:opacity-50 dark:hover:bg-amber-900/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                >
                  Edit
                </button>
              </li>
            ))}
          </ul>
        </>
      )}

      {openFile && (
        <div className="mt-2">
          <div className="flex items-center justify-between gap-2 pb-1 text-xs opacity-70">
            <span className="min-w-0 truncate">{openFile}</span>
            <div className="flex shrink-0 items-center gap-2">
              <span>
                {markerLines > 0
                  ? `${markerLines} conflict ${markerLines === 1 ? "marker" : "markers"}`
                  : "no markers"}
              </span>
              <button
                type="button"
                onClick={reloadFile}
                disabled={busy !== null}
                title="Read the file again after editing it outside Knitt"
                className="rounded px-1 hover:bg-amber-100 disabled:opacity-50 dark:hover:bg-amber-900/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                {busy === "reload" ? "Reloading…" : "Reload"}
              </button>
            </div>
          </div>
          <textarea
            value={content}
            onChange={(event) => setContent(event.target.value)}
            spellCheck={false}
            aria-label={`Resolve ${openFile}`}
            className="h-48 w-full rounded border border-neutral-300 bg-white p-2 font-mono text-xs dark:border-neutral-700 dark:bg-neutral-900"
          />
          <div className="mt-1 flex items-center gap-2">
            <button
              type="button"
              onClick={saveResolution}
              disabled={busy !== null || !dirty}
              className="rounded bg-neutral-900 px-2 py-1 text-xs text-white hover:bg-neutral-800 disabled:opacity-50 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              {busy === "save" ? "Saving…" : "Save resolution"}
            </button>
            <button
              type="button"
              onClick={() => setContent(loadedContent)}
              disabled={busy !== null || !dirty}
              className="rounded px-2 py-1 text-xs hover:bg-amber-100 disabled:opacity-50 dark:hover:bg-amber-900/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Discard edits
            </button>
          </div>
        </div>
      )}

      {error && (
        <p role="alert" className="mt-2 text-xs text-red-700 dark:text-red-400">
          {error}
        </p>
      )}

      {aborting && (
        <ConfirmDialog
          title={`Abandon the ${describe(state)}?`}
          description="The operation is cancelled and your branch is restored to where it was before it started. Resolved files and new edits are lost."
          confirmLabel={`Abandon ${describe(state)}`}
          busy={busy === "abort"}
          error={error}
          onConfirm={abort}
          onCancel={() => {
            setAborting(false);
            setError(null);
          }}
        />
      )}
    </div>
  );
}
