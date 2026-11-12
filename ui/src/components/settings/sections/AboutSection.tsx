import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-shell";
import logo from "../../../assets/logo.png";
import { SectionHeader, SettingRow, type SectionProps } from "../controls";
import { loadEnvironment, type EnvironmentInfo } from "../../../lib/diagnostics";

const LINKS = [
  { label: "Source code", url: "https://github.com/Tazorx7/knitt" },
  { label: "Report an issue", url: "https://github.com/Tazorx7/knitt/issues" },
  {
    label: "Licence (Apache-2.0)",
    url: "https://github.com/Tazorx7/knitt/blob/main/LICENSE",
  },
];

/** Key components and the licence each is distributed under. */
const COMPONENTS = [
  { name: "Tauri", licence: "MIT / Apache-2.0" },
  { name: "React", licence: "MIT" },
  { name: "Monaco Editor", licence: "MIT" },
  { name: "Tailwind CSS", licence: "MIT" },
  { name: "libgit2 (via git2)", licence: "GPL-2.0 with linking exception" },
  { name: "reqwest", licence: "MIT / Apache-2.0" },
];

export default function AboutSection(_props: SectionProps) {
  const [env, setEnv] = useState<EnvironmentInfo | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    loadEnvironment()
      .then(setEnv)
      .catch(() => setStatus("Could not read the application details."));
  }, []);

  async function openLink(url: string) {
    setStatus(null);
    try {
      await open(url);
    } catch {
      setStatus("Could not open that link in your browser.");
    }
  }

  const build = import.meta.env.DEV ? "Development build" : "Release build";

  return (
    <>
      <SectionHeader
        title="About"
        description="Version, build details and project links."
      />

      <div className="mb-6 flex items-center gap-4">
        <img src={logo} alt="" className="h-14 w-14 rounded-lg object-contain" />
        <div>
          <h2 className="text-lg font-semibold">Knitt</h2>
          <p className="text-xs opacity-70">
            {env ? `Version ${env.app_version} · ${build}` : build}
          </p>
        </div>
      </div>

      <SettingRow
        id="setting-about-build"
        title="Build"
        description="Use these details when reporting a problem."
      >
        <span className="text-xs opacity-80">
          {env ? `${env.os} ${env.arch}` : "Loading…"}
        </span>
      </SettingRow>

      <SettingRow title="Git backend" description="Embedded Git library used by Knitt.">
        <span className="text-xs opacity-80">{env?.git_backend ?? "Loading…"}</span>
      </SettingRow>

      <SettingRow
        title="Project links"
        description="Open the repository, issue tracker or licence in your browser."
      >
        <div className="flex flex-col items-end gap-1">
          {LINKS.map((link) => (
            <button
              key={link.url}
              type="button"
              onClick={() => openLink(link.url)}
              className="rounded px-2 py-0.5 text-sm text-blue-700 hover:bg-neutral-100 dark:text-blue-400 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              {link.label}
            </button>
          ))}
        </div>
      </SettingRow>

      {status && (
        <p role="status" className="mt-3 text-xs text-red-700 dark:text-red-400">
          {status}
        </p>
      )}

      <details className="mt-8 rounded border border-neutral-200 p-3 dark:border-neutral-800">
        <summary className="cursor-pointer text-sm font-semibold">
          Third-party notices
        </summary>
        <p className="mt-2 text-xs opacity-70">
          Knitt is built with these open-source components. Each remains under
          its own licence; the full dependency list is in `Cargo.lock` and
          `ui/package-lock.json` in the source repository.
        </p>
        <ul className="mt-3 space-y-1">
          {COMPONENTS.map((component) => (
            <li
              key={component.name}
              className="flex items-center justify-between gap-4 text-xs"
            >
              <span>{component.name}</span>
              <span className="opacity-70">{component.licence}</span>
            </li>
          ))}
        </ul>
      </details>
    </>
  );
}
