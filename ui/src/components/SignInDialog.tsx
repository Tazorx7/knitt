import { open } from "@tauri-apps/plugin-shell";
import { useDialogFocus } from "../hooks/useDialogFocus";

type SignInDialogProps = {
  host: string;
  onHostChange: (host: "github.com" | "enterprise") => void;
  customHost: string;
  onCustomHostChange: (value: string) => void;
  userCode: string | null;
  verificationUri: string | null;
  status: string;
  onContinue: () => void;
  onCancel: () => void;
};

/**
 * GitHub device-flow sign-in. Stays open until the user cancels or the flow
 * finishes, and keeps keyboard focus inside while it is shown.
 */
export default function SignInDialog({
  host,
  onHostChange,
  customHost,
  onCustomHostChange,
  userCode,
  verificationUri,
  status,
  onContinue,
  onCancel,
}: SignInDialogProps) {
  const dialogRef = useDialogFocus<HTMLDivElement>({ onDismiss: onCancel });

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          onCancel();
        }
      }}
    >
      <div
        ref={dialogRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby="sign-in-title"
        className="w-[420px] rounded-lg border border-neutral-200 bg-neutral-50 p-6 shadow-lg focus:outline-none dark:border-neutral-800 dark:bg-neutral-900"
      >
        {!userCode ? (
          <>
            <h2 id="sign-in-title" className="text-lg font-medium">
              Sign in to GitHub
            </h2>
            <div className="mt-4 flex flex-col gap-2">
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="radio"
                  checked={host === "github.com"}
                  onChange={() => onHostChange("github.com")}
                />
                GitHub.com
              </label>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="radio"
                  checked={host === "enterprise"}
                  onChange={() => onHostChange("enterprise")}
                />
                GitHub Enterprise Server
              </label>
              {host === "enterprise" && (
                <input
                  value={customHost}
                  onChange={(event) => onCustomHostChange(event.target.value)}
                  placeholder="git.example.com"
                  aria-label="Enterprise host"
                  className="mt-1 rounded border border-neutral-300 px-2 py-1 text-sm dark:border-neutral-700 dark:bg-neutral-800"
                />
              )}
            </div>
            <div className="mt-6 flex justify-end gap-2">
              <button
                type="button"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  onCancel();
                }}
                className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  onContinue();
                }}
                className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                Continue
              </button>
            </div>
            {status && (
              <div role="status" aria-live="polite" className="mt-3 text-xs opacity-70">
                {status}
              </div>
            )}
          </>
        ) : (
          <>
            <h2 id="sign-in-title" className="text-lg font-medium">
              Authorize this app
            </h2>
            <p className="mt-2 text-sm opacity-80">
              Enter this code in your browser
            </p>
            <div className="mt-4 flex items-center justify-center rounded bg-neutral-100 py-3 text-2xl font-mono tracking-widest dark:bg-neutral-800">
              {userCode}
            </div>
            <div className="mt-4 flex flex-col gap-2">
              <button
                type="button"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  if (verificationUri) open(verificationUri);
                }}
                className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                Open in browser
              </button>
              <button
                type="button"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  if (verificationUri) navigator.clipboard.writeText(verificationUri);
                }}
                className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                Copy verification URL
              </button>
            </div>
            <div role="status" aria-live="polite" className="mt-4 text-xs opacity-70">
              {status}
            </div>
            <div className="mt-4 flex justify-end">
              <button
                type="button"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  onCancel();
                }}
                className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                Cancel
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
