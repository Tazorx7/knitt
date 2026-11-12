import { SectionHeader, SettingRow, Toggle, type SectionProps } from "../controls";

export default function SafetySection({ settings, onChange }: SectionProps) {
  const recommended =
    settings.confirm_discard &&
    settings.confirm_tag_delete &&
    settings.confirm_stash_drop;

  return (
    <>
      <SectionHeader
        title="Safety"
        description="Confirmations before actions that can lose local work. Actions Knitt cannot perform yet are not listed."
      />

      <SettingRow
        id="setting-confirm-discard"
        title="Discard changes"
        description="Ask before restoring a file to its committed version, which discards uncommitted edits."
      >
        <Toggle
          checked={settings.confirm_discard}
          onChange={(confirm_discard) => onChange({ confirm_discard })}
          label="Confirm discarding changes"
        />
      </SettingRow>

      <SettingRow
        id="setting-confirm-tag-delete"
        title="Delete tag"
        description="Ask before deleting a tag. Pushed tags are not removed from the remote."
      >
        <Toggle
          checked={settings.confirm_tag_delete}
          onChange={(confirm_tag_delete) => onChange({ confirm_tag_delete })}
          label="Confirm deleting tags"
        />
      </SettingRow>

      <SettingRow
        id="setting-confirm-stash-drop"
        title="Drop stash"
        description="Ask before permanently removing the latest stash entry."
      >
        <Toggle
          checked={settings.confirm_stash_drop}
          onChange={(confirm_stash_drop) => onChange({ confirm_stash_drop })}
          label="Confirm dropping stashes"
        />
      </SettingRow>

      <div className="flex flex-wrap items-center gap-3 py-4">
        <button
          type="button"
          onClick={() =>
            onChange({
              confirm_discard: true,
              confirm_tag_delete: true,
              confirm_stash_drop: true,
            })
          }
          disabled={recommended}
          className="rounded border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
        >
          Restore recommended defaults
        </button>
        {recommended && (
          <span className="text-xs opacity-60">All confirmations are on.</span>
        )}
      </div>

      <p className="mt-2 text-xs opacity-70">
        Confirmation dialogs name the exact file, tag or stash they affect and
        never offer a global “never ask again” option. Reset, clean, force push
        and repository removal are not implemented, so they have no settings
        here.
      </p>
    </>
  );
}
