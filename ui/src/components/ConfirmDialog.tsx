import { useRef } from "react";
import { useDialogFocus } from "../hooks/useDialogFocus";

export type ConfirmDialogProps = {
  title: string;
  description: string;
  confirmLabel: string;
  /** Extra line naming the exact target (branch, file, tag). */
  target?: string;
  /** `danger` marks a destructive action; use `default` for reversible ones. */
  tone?: "danger" | "default";
  busy?: boolean;
  error?: string | null;
  onConfirm: () => void;
  onCancel: () => void;
};

/**
 * A focused confirmation dialog. Cancel receives initial focus so an
 * accidental Enter does not confirm a destructive action, Escape always
 * cancels, and focus stays inside the dialog until it closes.
 */
export default function ConfirmDialog({
  title,
  description,
  confirmLabel,
  target,
  tone = "danger",
  busy = false,
  error = null,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const dialogRef = useDialogFocus<HTMLDivElement>({
    onDismiss: onCancel,
    dismissDisabled: busy,
    initialFocus: cancelRef,
  });

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        ref={dialogRef}
        tabIndex={-1}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        aria-describedby="confirm-description"
        className="w-full max-w-md rounded-lg border border-neutral-200 bg-white p-4 shadow-xl focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
      >
        <h2 id="confirm-title" className="text-sm font-semibold">
          {title}
        </h2>
        <p id="confirm-description" className="mt-1 text-xs opacity-70">
          {description}
        </p>
        {target && (
          <p className="mt-2 break-words rounded bg-neutral-100 px-2 py-1 font-mono text-xs dark:bg-neutral-800">
            {target}
          </p>
        )}
        {error && (
          <p role="alert" className="mt-2 text-xs text-red-700 dark:text-red-400">
            {error}
          </p>
        )}
        <div className="mt-4 flex justify-end gap-2">
          <button
            ref={cancelRef}
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
            className={`rounded px-3 py-1 text-sm text-white disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
              tone === "danger"
                ? "bg-red-600 hover:bg-red-500"
                : "bg-[var(--accent)] hover:opacity-90"
            }`}
          >
            {busy ? "Working…" : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
