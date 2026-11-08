import { useEffect, useState } from "react";
import {
  SectionHeader,
  SettingRow,
  Segmented,
  TextInput,
  type SectionProps,
} from "../controls";
import type { ThemeMode } from "../../../lib/settings";

export default function AppearanceSection({ settings, onChange }: SectionProps) {
  const [tabWidth, setTabWidth] = useState(String(settings.diff_tab_width));

  useEffect(() => {
    setTabWidth(String(settings.diff_tab_width));
  }, [settings.diff_tab_width]);

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

  return (
    <>
      <SectionHeader
        title="Appearance"
        description="How Knitt looks. Changes apply immediately."
      />

      <SettingRow
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

      <SettingRow
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
    </>
  );
}
