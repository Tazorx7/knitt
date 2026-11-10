import { open } from "@tauri-apps/plugin-dialog";
import { SectionHeader, SettingRow, Toggle, type SectionProps } from "../controls";

export default function GeneralSection({ settings, onChange }: SectionProps) {
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

  return (
    <>
      <SectionHeader
        title="General"
        description="Startup behaviour and where Knitt looks for repositories."
      />

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

      <SettingRow
        id="setting-default-directory"
        title="Default repository directory"
        description="Starting folder for clone and create dialogs. Existing repositories are never moved."
      >
        <div className="flex items-center gap-2">
          <span className="max-w-64 truncate text-xs opacity-70" title={settings.default_repository_directory ?? undefined}>
            {settings.default_repository_directory ?? "Not set"}
          </span>
          <button
            type="button"
            onClick={chooseDirectory}
            className="rounded border border-neutral-300 px-2 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
          >
            Browse…
          </button>
          {settings.default_repository_directory && (
            <button
              type="button"
              onClick={() => onChange({ default_repository_directory: null })}
              className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
            >
              Clear
            </button>
          )}
        </div>
      </SettingRow>
    </>
  );
}
