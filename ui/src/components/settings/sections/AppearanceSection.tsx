import { useEffect, useState } from "react";
import {
  SectionHeader,
  SettingRow,
  Segmented,
  TextInput,
  Toggle,
  type SectionProps,
} from "../controls";
import type { ThemeMode } from "../../../lib/settings";

export default function AppearanceSection({ settings, onChange }: SectionProps) {
  const [tabWidth, setTabWidth] = useState(String(settings.diff_tab_width));
  const [fontSize, setFontSize] = useState(String(settings.diff_font_size));

  useEffect(() => {
    setTabWidth(String(settings.diff_tab_width));
  }, [settings.diff_tab_width]);

  useEffect(() => {
    setFontSize(String(settings.diff_font_size));
  }, [settings.diff_font_size]);

  function commitTabWidth() {
    const parsed = Number.parseInt(tabWidth, 10);
    const clamped = Number.isFinite(parsed)
      ? Math.min(16, Math.max(1, parsed))
      : settings.diff_tab_width;
    setTabWidth(String(clamped));
    if (clamped !== settings.diff_tab_width) {
      onChange({ diff_tab_width: clamped });
    }
  }

  function commitFontSize() {
    const parsed = Number.parseInt(fontSize, 10);
    const clamped = Number.isFinite(parsed)
      ? Math.min(24, Math.max(8, parsed))
      : settings.diff_font_size;
    setFontSize(String(clamped));
    if (clamped !== settings.diff_font_size) {
      onChange({ diff_font_size: clamped });
    }
  }

  return (
    <>
      <SectionHeader
        title="Appearance"
        description="How Knitt looks. Changes apply immediately."
      />

      <SettingRow
        id="setting-theme"
        title="Theme"
        description="System follows your desktop's light or dark preference."
      >
        <Segmented<ThemeMode>
          value={settings.theme}
          label="Theme"
          options={[
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark" },
            { value: "system", label: "System" },
          ]}
          onChange={(theme) => onChange({ theme })}
        />
      </SettingRow>

      <h2 className="mt-8 text-sm font-semibold">Diff view</h2>
      <p className="mt-1 text-xs opacity-70">
        Display only. These preferences never change file contents or line
        endings.
      </p>

      <SettingRow
        id="setting-diff-layout"
        title="Layout"
        description="Split shows the old and new file side by side."
      >
        <Segmented<"split" | "unified">
          value={settings.diff_side_by_side ? "split" : "unified"}
          label="Diff layout"
          options={[
            { value: "split", label: "Split" },
            { value: "unified", label: "Unified" },
          ]}
          onChange={(layout) => onChange({ diff_side_by_side: layout === "split" })}
        />
      </SettingRow>

      <SettingRow
        id="setting-diff-tab-width"
        title="Diff tab width"
        description="Spaces used to render tab characters in the diff viewer."
        htmlFor="diff-tab-width"
      >
        <TextInput
          id="diff-tab-width"
          type="number"
          min={1}
          max={16}
          value={tabWidth}
          onChange={(event) => setTabWidth(event.target.value)}
          onBlur={commitTabWidth}
          onKeyDown={(event) => {
            if (event.key === "Enter") commitTabWidth();
          }}
          className="w-20"
        />
      </SettingRow>

      <SettingRow
        id="setting-diff-font-size"
        title="Diff font size"
        description="Pixels for the diff viewer text."
        htmlFor="diff-font-size"
      >
        <TextInput
          id="diff-font-size"
          type="number"
          min={8}
          max={24}
          value={fontSize}
          onChange={(event) => setFontSize(event.target.value)}
          onBlur={commitFontSize}
          onKeyDown={(event) => {
            if (event.key === "Enter") commitFontSize();
          }}
          className="w-20"
        />
      </SettingRow>

      <SettingRow
        id="setting-diff-whitespace"
        title="Show whitespace changes"
        description="When off, changes that only touch whitespace are hidden from the diff."
      >
        <Toggle
          checked={!settings.diff_ignore_whitespace}
          onChange={(show) => onChange({ diff_ignore_whitespace: !show })}
          label="Show whitespace changes"
        />
      </SettingRow>

      <SettingRow
        id="setting-diff-wrap"
        title="Wrap long lines"
        description="Wrap diff lines instead of scrolling horizontally."
      >
        <Toggle
          checked={settings.diff_wrap}
          onChange={(diff_wrap) => onChange({ diff_wrap })}
          label="Wrap long lines"
        />
      </SettingRow>
    </>
  );
}
