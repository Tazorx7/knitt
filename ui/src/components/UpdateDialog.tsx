import { useRef } from "react";
import { useDialogFocus } from "../hooks/useDialogFocus";
import type { UpdateProgress } from "../lib/updates";

export type UpdateDialogProps = {
  version: string;
  notes: string | null;
  busy: boolean;
  progress: UpdateProgress | null;
  error: string | null;
  onInstall: () => void;
  onLater: () => void;
};

/** One decimal place, in mebibytes: the download is tens of megabytes. */
function mebibytes(bytes: number) {
  return (bytes / 1024 / 1024).toFixed(1);
}

/**
 * Offers a newer release. "Later" holds initial focus, because installing
 * restarts the app and that is not something to trigger with a stray Enter.
 */
export default function UpdateDialog({
  version,
  notes,
  busy,
  progress,
  error,
  onInstall,
  onLater,
}: UpdateDialogProps) {
  const laterRef = useRef<HTMLButtonElement | null>(null);
  const dialogRef = useDialogFocus<HTMLDivElement>({
    onDismiss: onLater,
    dismissDisabled: busy,
    initialFocus: laterRef,
  });

  const percent =
    progress?.total && progress.total > 0
      ? Math.min(100, Math.floor((progress.downloaded / progress.total) * 100))
      : null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        ref={dialogRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-title"
        aria-describedby="update-description"
        className="w-full max-w-md rounded-lg border border-neutral-200 bg-white p-4 shadow-xl focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
      >
        <h2 id="update-title" className="text-sm font-semibold">
          Version {version} is available
        </h2>
        <p id="update-description" className="mt-1 text-xs opacity-70">
          Knitt downloads the update and restarts when it is ready.
        </p>
        {notes && (
          <div className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-neutral-100 px-2 py-1 text-xs dark:bg-neutral-800">
            {notes}
          </div>
        )}
        {busy && (
          <p role="status" className="mt-2 text-xs opacity-70">
            {percent === null
              ? "Downloading…"
              : `Downloading… ${percent}% of ${mebibytes(progress?.total ?? 0)} MB`}
          </p>
        )}
        {error && (
          <p role="alert" className="mt-2 text-xs text-red-700 dark:text-red-400">
            {error}
          </p>
        )}
        <div className="mt-4 flex justify-end gap-2">
          <button
            ref={laterRef}
            type="button"
            onClick={onLater}
            disabled={busy}
            className="rounded px-3 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Later
          </button>
          <button
            type="button"
            onClick={onInstall}
            disabled={busy}
            className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            {busy ? "Installing…" : "Install and restart"}
          </button>
        </div>
      </div>
    </div>
  );
}
