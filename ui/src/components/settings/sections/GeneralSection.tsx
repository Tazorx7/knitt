import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  SectionHeader,
  SettingRow,
  Segmented,
  Toggle,
  type SectionProps,
} from "../controls";
import {
  forgetRecent,
  removeRecent,
  setAutostart,
  type Settings,
  type StartupView,
} from "../../../lib/settings";

export default function GeneralSection({ settings, onChange }: SectionProps) {
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function chooseDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose default repository directory",
    });
    if (typeof selected === "string") {
      onChange({ default_repository_directory: selected });
    }
  }

  async function toggleAutostart(enabled: boolean) {
    setBusy(true);
    setStatus(null);
    try {
      const actual = await setAutostart(enabled);
      onChange({ autostart: actual });
      if (actual !== enabled) {
        setStatus("The desktop did not accept the change.");
      }
    } catch {
      setStatus("Could not update launch at login.");
    } finally {
      setBusy(false);
    }
  }

  function applySettings(next: Settings) {
    onChange(next);
  }

  async function handleRemoveRecent(path: string) {
    setBusy(true);
    setStatus(null);
    try {
      applySettings(await removeRecent(path));
    } catch {
      setStatus("Could not update the recent list.");
    } finally {
      setBusy(false);
    }
  }

  async function handleForgetRecent() {
    setBusy(true);
    setStatus(null);
    try {
      applySettings(await forgetRecent());
    } catch {
      setStatus("Could not clear the recent list.");
    } finally {
      setBusy(false);
    }
  }

  const recent = settings.recent_repositories;

  return (
    <>
      <SectionHeader
        title="General"
        description="Startup behaviour and where Knitt looks for repositories."
      />

      <SettingRow
        id="setting-startup-view"
        title="Startup view"
        description="Open the last repository, or always start on the empty view."
      >
        <Segmented<StartupView>
          value={settings.startup_view}
          label="Startup view"
          options={[
            { value: "last", label: "Last repository" },
            { value: "empty", label: "Empty view" },
          ]}
          onChange={(startup_view) => onChange({ startup_view })}
        />
      </SettingRow>

      {settings.startup_view === "last" && (
        <SettingRow
          id="setting-restore-repository"
          title="Restore last repository"
          description="Reopen the repository you used last, if it still exists on disk."
        >
          <Toggle
            checked={settings.restore_last_repository}
            onChange={(restore_last_repository) => onChange({ restore_last_repository })}
            label="Restore last repository"
          />
        </SettingRow>
      )}

      <SettingRow
        id="setting-autostart"
        title="Launch at login"
        description="Adds a desktop autostart entry. You can remove it from your desktop's startup settings."
      >
        <Toggle
          checked={settings.autostart}
          onChange={toggleAutostart}
          label="Launch Knitt at login"
        />
      </SettingRow>

      <SettingRow
        id="setting-default-directory"
        title="Default repository directory"
        description="Starting folder for clone and create dialogs. Existing repositories are never moved."
      >
        <div className="flex items-center gap-2">
          <span
            className="max-w-64 truncate text-xs opacity-70"
            title={settings.default_repository_directory ?? undefined}
          >
            {settings.default_repository_directory ?? "Not set"}
          </span>
          <button
            type="button"
            onClick={chooseDirectory}
            className="rounded border border-neutral-300 px-2 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Browse…
          </button>
          {settings.default_repository_directory && (
            <button
              type="button"
              onClick={() => onChange({ default_repository_directory: null })}
              className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Clear
            </button>
          )}
        </div>
      </SettingRow>

      <SettingRow
        id="setting-recent-repositories"
        title="Recent repositories"
        description="Kept on this machine only. Removing an entry never deletes the repository from disk."
      >
        {recent.length === 0 ? (
          <span className="text-xs opacity-70">No recent repositories</span>
        ) : (
          <button
            type="button"
            onClick={handleForgetRecent}
            disabled={busy}
            className="rounded px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Forget recent history
          </button>
        )}
      </SettingRow>

      {recent.length > 0 && (
        <ul className="divide-y divide-neutral-200 dark:divide-neutral-800" aria-label="Recent repositories">
          {recent.map((path) => (
            <li key={path} className="flex items-center justify-between gap-3 py-2">
              <span className="min-w-0">
                <span className="block truncate text-sm">{path.split("/").pop() || path}</span>
                <span className="block truncate text-xs opacity-60" title={path}>
                  {path}
                </span>
              </span>
              <button
                type="button"
                onClick={() => handleRemoveRecent(path)}
                disabled={busy}
                aria-label={`Remove ${path} from the recent list`}
                className="shrink-0 rounded px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}

      {status && (
        <p role="status" className="mt-3 text-xs text-red-700 dark:text-red-400">
          {status}
        </p>
      )}
    </>
  );
}
