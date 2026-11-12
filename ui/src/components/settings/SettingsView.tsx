import { useEffect, useMemo, useState } from "react";
import type { Settings } from "../../lib/settings";
import GeneralSection from "./sections/GeneralSection";
import AccountsSection, { type AccountsSectionProps } from "./sections/AccountsSection";
import AppearanceSection from "./sections/AppearanceSection";
import GitSection from "./sections/GitSection";
import IntegrationsSection from "./sections/IntegrationsSection";
import AdvancedSection from "./sections/AdvancedSection";
import AboutSection from "./sections/AboutSection";
import AccessibilitySection from "./sections/AccessibilitySection";

const SECTIONS = [
  { id: "general", label: "General" },
  { id: "accounts", label: "Accounts" },
  { id: "integrations", label: "Integrations" },
  { id: "git", label: "Git" },
  { id: "appearance", label: "Appearance" },
  { id: "advanced", label: "Advanced" },
  { id: "accessibility", label: "Accessibility" },
  { id: "about", label: "About" },
] as const;

type SectionId = (typeof SECTIONS)[number]["id"];

type SearchEntry = {
  section: SectionId;
  title: string;
  description: string;
  keywords: string[];
  anchor: string;
};

/** Search names, descriptions and synonyms. Never includes secret values. */
const SEARCH_INDEX: SearchEntry[] = [
  {
    section: "general",
    title: "Restore last repository",
    description: "Reopen the repository you used last, if it still exists.",
    keywords: ["startup", "reopen", "last", "repository", "restore"],
    anchor: "setting-restore-repository",
  },
  {
    section: "general",
    title: "Startup view",
    description: "Open the last repository or always start on the empty view.",
    keywords: ["startup", "start", "launch", "last repository", "empty"],
    anchor: "setting-startup-view",
  },
  {
    section: "general",
    title: "Launch at login",
    description: "Add a desktop autostart entry for Knitt.",
    keywords: ["autostart", "login", "startup", "boot", "launch"],
    anchor: "setting-autostart",
  },
  {
    section: "general",
    title: "Recent repositories",
    description: "Review or forget repositories opened recently on this machine.",
    keywords: ["recent", "history", "repositories", "forget", "clear"],
    anchor: "setting-recent-repositories",
  },
  {
    section: "general",
    title: "Default repository directory",
    description: "Starting folder for clone and create dialogs.",
    keywords: ["folder", "path", "directory", "clone", "repositories"],
    anchor: "setting-default-directory",
  },
  {
    section: "accounts",
    title: "GitHub accounts",
    description: "Sign in, sign out, switch accounts or add GitHub Enterprise.",
    keywords: [
      "github",
      "sign in",
      "sign out",
      "login",
      "logout",
      "token",
      "enterprise",
      "device",
    ],
    anchor: "setting-accounts",
  },
  {
    section: "integrations",
    title: "External editor",
    description: "Open repositories in VS Code, Zed, Sublime or a custom editor.",
    keywords: [
      "editor",
      "vscode",
      "vscodium",
      "zed",
      "sublime",
      "idea",
      "executable",
      "command",
    ],
    anchor: "setting-editor",
  },
  {
    section: "integrations",
    title: "Terminal",
    description: "Open a terminal in the repository folder.",
    keywords: [
      "terminal",
      "shell",
      "konsole",
      "kitty",
      "alacritty",
      "gnome-terminal",
      "executable",
    ],
    anchor: "setting-terminal",
  },
  {
    section: "integrations",
    title: "File manager",
    description: "Open the repository folder in the system file manager.",
    keywords: ["file manager", "folder", "open folder", "xdg-open"],
    anchor: "setting-file-manager",
  },
  {
    section: "git",
    title: "Git author name",
    description: "Global user.name used for commits.",
    keywords: ["name", "author", "identity", "user.name", "commit"],
    anchor: "setting-git-name",
  },
  {
    section: "git",
    title: "Git author email",
    description: "Global user.email used for commits.",
    keywords: ["email", "author", "identity", "user.email", "commit"],
    anchor: "setting-git-email",
  },
  {
    section: "git",
    title: "Default branch name",
    description: "Branch name used when Knitt creates a repository.",
    keywords: ["branch", "main", "master", "repository creation"],
    anchor: "setting-default-branch",
  },
  {
    section: "git",
    title: "Repository commit identity",
    description:
      "Override the author name and email for commits in the open repository only.",
    keywords: [
      "repository",
      "local",
      "override",
      "author",
      "identity",
      "name",
      "email",
      "per repository",
    ],
    anchor: "setting-repo-git-name",
  },
  {
    section: "git",
    title: "Pull strategy",
    description: "Merge, rebase or fast-forward only when pulling remote changes.",
    keywords: ["pull", "merge", "rebase", "fast forward", "ff-only", "sync"],
    anchor: "setting-pull-strategy",
  },
  {
    section: "git",
    title: "Push mode",
    description: "Push the current branch name or the branch it tracks.",
    keywords: ["push", "upstream", "tracked", "remote", "sync"],
    anchor: "setting-push-mode",
  },
  {
    section: "git",
    title: "Prune on fetch",
    description: "Remove remote-tracking branches that no longer exist on the remote.",
    keywords: ["fetch", "prune", "stale", "remote branches", "sync"],
    anchor: "setting-fetch-prune",
  },
  {
    section: "appearance",
    title: "Theme",
    description: "Light, dark or follow the system.",
    keywords: ["theme", "dark", "light", "system", "colour", "color"],
    anchor: "setting-theme",
  },
  {
    section: "appearance",
    title: "Accent colour",
    description: "Preset colour for focus rings and primary actions.",
    keywords: ["accent", "colour", "color", "theme", "focus", "highlight"],
    anchor: "setting-accent",
  },
  {
    section: "appearance",
    title: "Density",
    description: "Comfortable or compact spacing in settings and side panels.",
    keywords: ["density", "compact", "comfortable", "spacing", "padding"],
    anchor: "setting-density",
  },
  {
    section: "appearance",
    title: "Diff layout",
    description: "Split or unified diff view.",
    keywords: ["diff", "split", "unified", "side by side", "layout"],
    anchor: "setting-diff-layout",
  },
  {
    section: "appearance",
    title: "Diff font size",
    description: "Text size in the diff viewer.",
    keywords: ["diff", "font", "size", "text", "monospace"],
    anchor: "setting-diff-font-size",
  },
  {
    section: "appearance",
    title: "Show whitespace changes",
    description: "Include whitespace-only changes in the diff.",
    keywords: ["diff", "whitespace", "spaces", "tabs", "indent"],
    anchor: "setting-diff-whitespace",
  },
  {
    section: "appearance",
    title: "Wrap long lines",
    description: "Wrap diff lines instead of scrolling horizontally.",
    keywords: ["diff", "wrap", "long lines", "word wrap"],
    anchor: "setting-diff-wrap",
  },
  {
    section: "appearance",
    title: "Diff tab width",
    description: "Spaces used to render tab characters in the diff viewer.",
    keywords: ["tab", "width", "diff", "indent", "spaces"],
    anchor: "setting-diff-tab-width",
  },
  {
    section: "advanced",
    title: "Logs",
    description: "Open the log folder or clear Knitt's diagnostic logs.",
    keywords: ["logs", "log file", "diagnostics", "troubleshooting", "debug"],
    anchor: "setting-advanced-logs",
  },
  {
    section: "advanced",
    title: "Diagnostic report",
    description: "Review and save a redacted report for bug reports.",
    keywords: [
      "diagnostics",
      "report",
      "version",
      "environment",
      "support",
      "bug report",
    ],
    anchor: "setting-advanced-report",
  },
  {
    section: "advanced",
    title: "Reset settings",
    description: "Restore default preferences without touching repositories or credentials.",
    keywords: ["reset", "defaults", "restore", "clear settings"],
    anchor: "setting-advanced-reset",
  },
  {
    section: "about",
    title: "Version and build",
    description: "Knitt version, build channel and the embedded Git backend.",
    keywords: ["about", "version", "build", "platform", "git version"],
    anchor: "setting-about-build",
  },
  {
    section: "about",
    title: "Third-party notices",
    description: "Open-source components and their licences.",
    keywords: ["licence", "license", "third party", "open source", "credits"],
    anchor: "setting-about-build",
  },
  {
    section: "accessibility",
    title: "Keyboard shortcuts",
    description: "Enable or disable global shortcuts and review the shortcut list.",
    keywords: ["keyboard", "shortcuts", "keys", "hotkeys", "accessibility"],
    anchor: "setting-keyboard-shortcuts",
  },
  {
    section: "accessibility",
    title: "Reduce motion",
    description: "Turn off non-essential animation, or follow the system setting.",
    keywords: ["motion", "animation", "reduce", "accessibility", "reduced motion"],
    anchor: "setting-reduce-motion",
  },
];

function sectionLabel(id: SectionId): string {
  return SECTIONS.find((section) => section.id === id)?.label ?? id;
}

export default function SettingsView({
  settings,
  onChange,
  onClose,
  accounts,
  repoPath,
  onReset,
}: {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onClose: () => void;
  accounts: AccountsSectionProps;
  repoPath: string | null;
  onReset: () => Promise<void>;
}) {
  const [active, setActive] = useState<SectionId>("general");
  const [query, setQuery] = useState("");
  const [pendingAnchor, setPendingAnchor] = useState<string | null>(null);

  const results = useMemo(() => {
    const tokens = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) return [];
    return SEARCH_INDEX.filter((entry) => {
      const haystack = [
        entry.title,
        entry.description,
        sectionLabel(entry.section),
        ...entry.keywords,
      ]
        .join(" ")
        .toLowerCase();
      return tokens.every((token) => haystack.includes(token));
    });
  }, [query]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      if (query) {
        event.preventDefault();
        setQuery("");
        setPendingAnchor(null);
      } else {
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, query]);

  // Runs after the target section renders, so the control exists before we
  // scroll to and focus it.
  useEffect(() => {
    if (!pendingAnchor) return;
    const target = document.getElementById(pendingAnchor);
    if (target) {
      target.scrollIntoView({ block: "center" });
      const focusable = target.querySelector<HTMLElement>(
        'input, select, textarea, button, [tabindex]:not([tabindex="-1"])',
      );
      focusable?.focus({ preventScroll: true });
    }
    setPendingAnchor(null);
  }, [active, pendingAnchor]);

  function openResult(entry: SearchEntry) {
    setActive(entry.section);
    setQuery("");
    setPendingAnchor(entry.anchor);
  }

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
            className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            aria-label="Close settings"
          >
            ✕
          </button>
        </div>

        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && results[0]) {
              event.preventDefault();
              openResult(results[0]);
            }
          }}
          placeholder="Search settings"
          aria-label="Search settings"
          className="mb-3 w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] dark:border-neutral-700 dark:bg-neutral-900"
        />

        {query.trim() ? (
          <div className="min-h-0 overflow-y-auto">
            <p className="px-2 pb-1 text-xs opacity-60" role="status">
              {results.length === 0
                ? "No matching settings"
                : `${results.length} ${results.length === 1 ? "setting" : "settings"}`}
            </p>
            {results.length === 0 ? (
              <p className="px-2 py-1 text-sm opacity-70">
                Nothing matches “{query.trim()}”.
              </p>
            ) : (
              <ul className="flex flex-col gap-0.5" aria-label="Search results">
                {results.map((entry) => (
                  <li key={entry.anchor}>
                    <button
                      type="button"
                      onClick={() => openResult(entry)}
                      className="w-full rounded px-2 py-1.5 text-left hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                    >
                      <span className="block text-sm">{entry.title}</span>
                      <span className="block text-xs opacity-60">
                        {sectionLabel(entry.section)}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ) : (
          <ul className="flex flex-col gap-1">
            {SECTIONS.map((section) => (
              <li key={section.id}>
                <button
                  type="button"
                  onClick={() => setActive(section.id)}
                  aria-current={active === section.id ? "page" : undefined}
                  className={`w-full rounded px-2 py-1 text-left text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)] ${
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
        )}
      </nav>

      <div className="min-h-0 flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-2xl">
          {active === "general" && (
            <GeneralSection settings={settings} onChange={onChange} />
          )}
          {active === "accounts" && <AccountsSection {...accounts} />}
          {active === "integrations" && (
            <IntegrationsSection
              settings={settings}
              onChange={onChange}
              repoPath={repoPath}
            />
          )}
          {active === "git" && (
            <GitSection settings={settings} onChange={onChange} repoPath={repoPath} />
          )}
          {active === "appearance" && (
            <AppearanceSection settings={settings} onChange={onChange} />
          )}
          {active === "advanced" && (
            <AdvancedSection settings={settings} onChange={onChange} onReset={onReset} />
          )}
          {active === "accessibility" && (
            <AccessibilitySection settings={settings} onChange={onChange} />
          )}
          {active === "about" && (
            <AboutSection settings={settings} onChange={onChange} />
          )}
        </div>
      </div>
    </div>
  );
}
