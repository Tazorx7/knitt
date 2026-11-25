import { useEffect, useRef, useState } from "react";
import { useDialogFocus } from "../hooks/useDialogFocus";
import { quitApp, uninstallKnitt, type UninstallPlan } from "../lib/uninstall";

type Phase = "confirm" | "running" | "done";

/**
 * Confirms an uninstall and shows exactly what goes: the package or AppImage,
 * every file Knitt wrote, and the saved GitHub tokens. The password dialog is
 * the system's, raised by polkit for the package manager.
 */
export default function UninstallDialog({
  plan,
  onCancel,
}: {
  plan: UninstallPlan;
  onCancel: () => void;
}) {
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const [phase, setPhase] = useState<Phase>("confirm");
  const [error, setError] = useState<string | null>(null);
  const [cancelled, setCancelled] = useState(false);
  const [failures, setFailures] = useState<string[]>([]);

  const busy = phase === "running";
  const finished = phase === "done";

  const dialogRef = useDialogFocus<HTMLDivElement>({
    onDismiss: onCancel,
    dismissDisabled: busy || finished,
    initialFocus: cancelRef,
  });

  // The package is gone, so the window closes itself rather than linger.
  useEffect(() => {
    if (!finished) return;
    const timer = window.setTimeout(() => {
      void quitApp();
    }, 2500);
    return () => window.clearTimeout(timer);
  }, [finished]);

  async function confirm() {
    setPhase("running");
    setError(null);
    setCancelled(false);
    try {
      const outcome = await uninstallKnitt();
      if (outcome.cancelled) {
        setCancelled(true);
        setPhase("confirm");
        return;
      }
      setFailures(outcome.failures);
      setPhase("done");
    } catch (reason) {
      setError(
        typeof reason === "string" ? reason : "Could not uninstall Knitt.",
      );
      setPhase("confirm");
    }
  }

  const title = finished ? "Knitt has been uninstalled" : "Uninstall Knitt";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        ref={dialogRef}
        tabIndex={-1}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="uninstall-title"
        aria-describedby="uninstall-description"
        className="w-full max-w-lg rounded-lg border border-neutral-200 bg-white p-4 shadow-xl focus:outline-none dark:border-neutral-700 dark:bg-neutral-900"
      >
        <h2 id="uninstall-title" className="text-sm font-semibold">
          {title}
        </h2>

        {finished ? (
          <>
            <p id="uninstall-description" className="mt-1 text-xs opacity-70">
              Knitt and everything it saved on this machine are gone. This
              window will close on its own.
            </p>
            {failures.length > 0 && (
              <div
                role="alert"
                className="mt-3 rounded border border-red-300 bg-red-50 p-2 text-xs text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200"
              >
                <p className="font-medium">
                  These steps did not finish. Remove them by hand:
                </p>
                <ul className="mt-1 list-disc pl-4">
                  {failures.map((failure) => (
                    <li key={failure}>{failure}</li>
                  ))}
                </ul>
              </div>
            )}
          </>
        ) : (
          <>
            <p id="uninstall-description" className="mt-1 text-xs opacity-70">
              This cannot be undone. Everything below is removed from this
              machine.
            </p>

            <div className="mt-3 max-h-64 overflow-y-auto rounded border border-neutral-200 dark:border-neutral-800">
              <section className="border-b border-neutral-200 p-2 dark:border-neutral-800">
                <h3 className="text-xs font-semibold uppercase tracking-wide opacity-60">
                  System
                </h3>
                <ul className="mt-1 space-y-1 text-xs">
                  {plan.system_items.map((item) => (
                    <li key={item}>{item}</li>
                  ))}
                </ul>
                <p className="mt-1 text-[11px] opacity-60">
                  Install detected as: {plan.kind_label}
                </p>
              </section>

              <section className="border-b border-neutral-200 p-2 dark:border-neutral-800">
                <h3 className="text-xs font-semibold uppercase tracking-wide opacity-60">
                  Your data
                </h3>
                {plan.data_items.length === 0 ? (
                  <p className="mt-1 text-xs opacity-70">
                    No saved data was found.
                  </p>
                ) : (
                  <ul className="mt-1 space-y-1">
                    {plan.data_items.map((item) => (
                      <li key={item.path} className="text-xs">
                        <span className="break-all font-mono text-[11px]">
                          {item.path}
                        </span>
                        <span className="block opacity-60">
                          {item.description}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>

              <section className="p-2">
                <h3 className="text-xs font-semibold uppercase tracking-wide opacity-60">
                  Accounts
                </h3>
                <p className="mt-1 text-xs">
                  {plan.keyring_accounts === 0
                    ? "No GitHub tokens are stored."
                    : `${plan.keyring_accounts} saved GitHub ${
                        plan.keyring_accounts === 1 ? "token" : "tokens"
                      } will be deleted from the system keyring.`}
                </p>
              </section>
            </div>

            <p className="mt-3 text-xs opacity-70">
              {plan.needs_password
                ? "Your system will ask for your password before the package is removed."
                : "No password is needed: nothing outside your home folder is changed."}
            </p>

            {cancelled && (
              <p role="status" className="mt-2 text-xs opacity-70">
                Uninstall cancelled. Nothing was changed.
              </p>
            )}

            {error && (
              <p
                role="alert"
                className="mt-2 text-xs text-red-700 dark:text-red-400"
              >
                {error}
              </p>
            )}
          </>
        )}

        <div className="mt-4 flex justify-end gap-2">
          {finished ? (
            <button
              type="button"
              onClick={() => void quitApp()}
              className="rounded px-3 py-1 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Close now
            </button>
          ) : (
            <>
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
                onClick={() => void confirm()}
                disabled={busy}
                className="rounded bg-red-600 px-3 py-1 text-sm text-white hover:bg-red-500 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                {busy ? "Removing…" : "Uninstall Knitt"}
              </button>
            </>
          )}
        </div>

        {busy && plan.needs_password && (
          <p role="status" className="mt-2 text-right text-xs opacity-70">
            Waiting for the password dialog…
          </p>
        )}
      </div>
    </div>
  );
}
