import { SectionHeader, SettingRow, Toggle, type SectionProps } from "../controls";

const SHORTCUTS: { keys: string; action: string }[] = [
  { keys: "Ctrl+O", action: "Open a repository" },
  { keys: "Ctrl+,", action: "Open or close settings" },
  { keys: "Ctrl+Enter", action: "Commit staged changes" },
  { keys: "Ctrl+Shift+P", action: "Push the current branch" },
  { keys: "Ctrl+Shift+F", action: "Fetch from the remote" },
  { keys: "Escape", action: "Close settings or clear the settings search" },
];

export default function AccessibilitySection({ settings, onChange }: SectionProps) {
  return (
    <>
      <SectionHeader
        title="Accessibility"
        description="Keyboard access and motion. Status messages use text, not colour alone."
      />

      <SettingRow
        id="setting-keyboard-shortcuts"
        title="Keyboard shortcuts"
        description="Global shortcuts such as Ctrl+O and Ctrl+Enter. Typing in fields always works."
      >
        <Toggle
          checked={settings.keyboard_shortcuts}
          onChange={(keyboard_shortcuts) => onChange({ keyboard_shortcuts })}
          label="Enable keyboard shortcuts"
        />
      </SettingRow>

      <SettingRow
        id="setting-reduce-motion"
        title="Reduce motion"
        description="Turn off non-essential animation. When off, Knitt follows your system setting."
      >
        <Toggle
          checked={settings.reduce_motion}
          onChange={(reduce_motion) => onChange({ reduce_motion })}
          label="Reduce motion"
        />
      </SettingRow>

      <h2 className="mt-8 text-sm font-semibold">Shortcut reference</h2>
      <p className="mt-1 text-xs opacity-70">
        Shortcuts are fixed for now; a shortcut editor is not implemented.
      </p>
      <table className="mt-3 w-full text-sm">
        <caption className="sr-only">Keyboard shortcuts</caption>
        <thead>
          <tr className="border-b border-neutral-200 text-left text-xs uppercase tracking-wide opacity-60 dark:border-neutral-800">
            <th scope="col" className="py-2 font-medium">
              Keys
            </th>
            <th scope="col" className="py-2 font-medium">
              Action
            </th>
          </tr>
        </thead>
        <tbody>
          {SHORTCUTS.map((shortcut) => (
            <tr
              key={shortcut.keys}
              className="border-b border-neutral-200 last:border-0 dark:border-neutral-800"
            >
              <th scope="row" className="py-2 pr-4 text-left font-mono text-xs font-normal">
                {shortcut.keys}
              </th>
              <td className="py-2 text-xs opacity-80">{shortcut.action}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <p className="mt-4 text-xs opacity-70">
        All settings are reachable with Tab and the arrow keys, and every
        control exposes a focus ring. Diff additions and deletions keep their
        <span className="mx-1 font-mono">+</span> and
        <span className="mx-1 font-mono">−</span> markers, so they stay
        distinguishable without colour.
      </p>
    </>
  );
}
