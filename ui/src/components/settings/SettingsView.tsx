import { useEffect, useState } from "react";
import type { Settings } from "../../lib/settings";
import GeneralSection from "./sections/GeneralSection";
import AccountsSection, { type AccountsSectionProps } from "./sections/AccountsSection";
import AppearanceSection from "./sections/AppearanceSection";
import GitSection from "./sections/GitSection";

const SECTIONS = [
  { id: "general", label: "General" },
  { id: "accounts", label: "Accounts" },
  { id: "git", label: "Git" },
  { id: "appearance", label: "Appearance" },
] as const;

type SectionId = (typeof SECTIONS)[number]["id"];

export default function SettingsView({
  settings,
  onChange,
  onClose,
  accounts,
}: {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onClose: () => void;
  accounts: AccountsSectionProps;
}) {
  const [active, setActive] = useState<SectionId>("general");

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div className="flex h-full w-full min-h-0">
      <nav
        aria-label="Settings sections"
        className="flex w-56 shrink-0 flex-col border-r border-neutral-200 p-3 dark:border-neutral-800"
      >
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold">Settings</h2>
          <button
            type="button"
            onClick={onClose}
            className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
            aria-label="Close settings"
          >
            ✕
          </button>
        </div>
        <ul className="flex flex-col gap-1">
          {SECTIONS.map((section) => (
            <li key={section.id}>
              <button
                type="button"
                onClick={() => setActive(section.id)}
                aria-current={active === section.id ? "page" : undefined}
                className={`w-full rounded px-2 py-1 text-left text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 ${
                  active === section.id
                    ? "bg-neutral-200 font-medium dark:bg-neutral-800"
                    : "hover:bg-neutral-100 dark:hover:bg-neutral-900"
                }`}
              >
                {section.label}
              </button>
            </li>
          ))}
        </ul>
      </nav>

      <div className="min-h-0 flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-2xl">
          {active === "general" && (
            <GeneralSection settings={settings} onChange={onChange} />
          )}
          {active === "accounts" && <AccountsSection {...accounts} />}
          {active === "git" && <GitSection settings={settings} onChange={onChange} />}
          {active === "appearance" && (
            <AppearanceSection settings={settings} onChange={onChange} />
          )}
        </div>
      </div>
    </div>
  );
}
