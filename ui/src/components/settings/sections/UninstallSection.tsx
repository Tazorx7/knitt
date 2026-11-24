import { useEffect, useState } from "react";
import { SectionHeader } from "../controls";
import UninstallDialog from "../../UninstallDialog";
import { loadUninstallPlan, type UninstallPlan } from "../../../lib/uninstall";

/**
 * The uninstall entry point. The dialog lists exactly what is removed, and
 * repositories are never part of it: only Knitt's own files and settings.
 */
export default function UninstallSection() {
  const [plan, setPlan] = useState<UninstallPlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let active = true;
    loadUninstallPlan()
      .then((loaded) => {
        if (active) setPlan(loaded);
      })
      .catch(() => {
        if (active) setError("Could not read what would be removed.");
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <>
      <SectionHeader
        title="Uninstall"
        description="Remove Knitt and everything it saved on this machine."
        id="setting-uninstall"
      />

      <div className="rounded-lg border border-red-300 p-4 dark:border-red-900">
        <h2 className="text-sm font-semibold">Remove Knitt</h2>
        <p className="mt-1 text-xs opacity-70">
          Your repositories are never touched. Only the installed package or
          AppImage, Knitt's settings, logs, cached web content and saved GitHub
          tokens are removed.
        </p>

        <p className="mt-2 text-xs">
          {plan
            ? `Detected install: ${plan.kind_label}`
            : error ?? "Reading the install details…"}
        </p>

        <button
          type="button"
          disabled={!plan}
          onClick={() => setOpen(true)}
          className="mt-3 rounded bg-red-600 px-3 py-1 text-sm text-white hover:bg-red-500 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
        >
          Uninstall Knitt…
        </button>

        {plan?.needs_password && (
          <p className="mt-2 text-[11px] opacity-60">
            Removing the package needs your password; the system asks for it.
          </p>
        )}
      </div>

      {open && plan && (
        <UninstallDialog plan={plan} onCancel={() => setOpen(false)} />
      )}
    </>
  );
}
