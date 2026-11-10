import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { SectionHeader, SettingRow, TextInput, type SectionProps } from "../controls";
import type { IntegrationConfig } from "../../../lib/settings";
import {
  defaultTestTarget,
  detectIntegrations,
  openInFileManager,
  openIntegration,
  testIntegration,
  validateExecutable,
  type DetectedApp,
} from "../../../lib/integrations";

type Status = { kind: "ok" | "error"; text: string } | null;
type IntegrationKind = "editor" | "terminal";

const CUSTOM = "__custom__";

const textButton =
  "rounded border border-neutral-300 px-2 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 disabled:opacity-50";
const primaryButton =
  "rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 disabled:opacity-50 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500";

export default function IntegrationsSection({
  settings,
  onChange,
  repoPath,
}: SectionProps & { repoPath: string | null }) {
  const [detected, setDetected] = useState<{ editors: DetectedApp[]; terminals: DetectedApp[] }>({
    editors: [],
    terminals: [],
  });
  const [detectFailed, setDetectFailed] = useState(false);
  const [fileStatus, setFileStatus] = useState<Status>(null);
  const [openingFolder, setOpeningFolder] = useState(false);

  useEffect(() => {
    let active = true;
    detectIntegrations()
      .then((result) => {
        if (active) setDetected(result);
      })
      .catch(() => {
        if (active) setDetectFailed(true);
      });
    return () => {
      active = false;
    };
  }, []);

  async function openFolder() {
    if (!repoPath) return;
    setOpeningFolder(true);
    setFileStatus(null);
    try {
      await openInFileManager(repoPath);
      setFileStatus({ kind: "ok", text: "Opened with the system file manager." });
    } catch (error) {
      setFileStatus({
        kind: "error",
        text: typeof error === "string" ? error : "Could not open the file manager.",
      });
    } finally {
      setOpeningFolder(false);
    }
  }

  return (
    <>
      <SectionHeader
        title="Integrations"
        description="Choose the applications Knitt may open for you. Launches run directly, never through a shell."
      />

      <IntegrationControl
        kind="editor"
        title="External editor"
        description="Opens a repository folder in an editor. Detected applications are those installed on this system."
        config={settings.external_editor}
        detected={detected.editors}
        defaultArguments="%TARGET_PATH%"
        onChange={(external_editor) => onChange({ external_editor })}
        target={repoPath}
      />

      <div className="mt-10">
        <IntegrationControl
          kind="terminal"
          title="Terminal"
          description="Opens a terminal in a repository folder. Each detected terminal uses its own working-directory flag."
          config={settings.terminal}
          detected={detected.terminals}
          defaultArguments="--working-directory=%TARGET_PATH%"
          onChange={(terminal) => onChange({ terminal })}
          target={repoPath}
        />
      </div>

      <div className="mt-10">
        <h2 id="setting-file-manager" className="text-sm font-semibold">File manager</h2>
        <p className="mt-1 text-xs opacity-70">
          Opens the repository folder with the desktop's registered file manager.
          Knitt does not assume a specific file manager is installed.
        </p>

        <SettingRow
          title="Repository folder"
          description={
            repoPath
              ? "Uses the system's file-opening mechanism for the current folder."
              : "Open a repository first to use this action."
          }
        >
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={openFolder}
              disabled={!repoPath || openingFolder}
              className={primaryButton}
            >
              {openingFolder ? "Opening…" : "Open folder"}
            </button>
            {fileStatus && (
              <span
                role="status"
                className={`text-xs ${
                  fileStatus.kind === "ok"
                    ? "text-green-700 dark:text-green-400"
                    : "text-red-700 dark:text-red-400"
                }`}
              >
                {fileStatus.text}
              </span>
            )}
          </div>
        </SettingRow>
      </div>

      {detectFailed && (
        <p className="mt-6 text-xs text-amber-700 dark:text-amber-400">
          Knitt could not scan for installed applications. You can still enter a
          custom executable path.
        </p>
      )}
    </>
  );
}

function IntegrationControl({
  kind,
  title,
  description,
  config,
  detected,
  defaultArguments,
  onChange,
  target,
}: {
  kind: IntegrationKind;
  title: string;
  description: string;
  config: IntegrationConfig;
  detected: DetectedApp[];
  defaultArguments: string;
  onChange: (config: IntegrationConfig) => void;
  target: string | null;
}) {
  const [executable, setExecutable] = useState(config.executable ?? "");
  const [args, setArgs] = useState(config.arguments);
  const [resolved, setResolved] = useState<string | null>(config.executable);
  const [status, setStatus] = useState<Status>(null);
  const [busy, setBusy] = useState(false);
  const id = `${kind}-integration`;

  useEffect(() => {
    setExecutable(config.executable ?? "");
    setArgs(config.arguments);
    setResolved(config.executable);
  }, [config.executable, config.arguments]);

  const matchesDetected = detected.some((app) => app.path === executable);
  const selectValue = executable === "" ? "" : matchesDetected ? executable : CUSTOM;

  function persist(next: IntegrationConfig) {
    if (next.executable === config.executable && next.arguments === config.arguments) {
      return;
    }
    onChange(next);
  }

  async function applyExecutable(value: string) {
    const trimmed = value.trim();
    if (!trimmed) {
      setResolved(null);
      persist({ executable: null, arguments: args });
      return;
    }
    try {
      const path = await validateExecutable(trimmed);
      setExecutable(path);
      setResolved(path);
      setStatus(null);
      persist({ executable: path, arguments: args });
    } catch (error) {
      setResolved(null);
      setStatus({
        kind: "error",
        text:
          typeof error === "string"
            ? error
            : "That executable could not be used. Check the path and permissions.",
      });
    }
  }

  async function browse() {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        title: `Choose a ${kind === "editor" ? "editor" : "terminal"} executable`,
      });
      if (typeof picked === "string") {
        setExecutable(picked);
        await applyExecutable(picked);
      }
    } catch {
      setStatus({ kind: "error", text: "The file picker is not available." });
    }
  }

  function choose(value: string) {
    if (value === CUSTOM) {
      setExecutable("");
      setResolved(null);
      return;
    }
    if (value === "") {
      setExecutable("");
      setResolved(null);
      persist({ executable: null, arguments: args });
      return;
    }
    const app = detected.find((entry) => entry.path === value);
    if (app) {
      setExecutable(app.path);
      setResolved(app.path);
      setArgs(app.arguments);
      persist({ executable: app.path, arguments: app.arguments });
    }
  }

  function commitArguments() {
    const value = args.trim() || defaultArguments;
    setArgs(value);
    persist({ executable: config.executable, arguments: value });
  }

  async function runTest() {
    if (!executable.trim()) {
      setStatus({
        kind: "error",
        text: kind === "editor" ? "Choose an editor first." : "Choose a terminal first.",
      });
      return;
    }
    setBusy(true);
    setStatus(null);
    try {
      const folder = target ?? (await defaultTestTarget());
      await testIntegration(kind, executable, args, folder);
      setStatus({
        kind: "ok",
        text: `Test launch started. ${kind === "editor" ? "The editor" : "The terminal"} should open ${folder}.`,
      });
    } catch (error) {
      setStatus({
        kind: "error",
        text: typeof error === "string" ? error : "The application could not be started.",
      });
    } finally {
      setBusy(false);
    }
  }

  async function openSaved() {
    if (!target) return;
    setBusy(true);
    setStatus(null);
    try {
      await openIntegration(kind, target);
    } catch (error) {
      setStatus({
        kind: "error",
        text: typeof error === "string" ? error : "The application could not be started.",
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <h2 id={`setting-${kind}`} className="text-sm font-semibold">{title}</h2>
      <p className="mt-1 text-xs opacity-70">{description}</p>

      <SettingRow
        title="Application"
        description="Detected applications only appear when installed."
        htmlFor={`${id}-app`}
      >
        <select
          id={`${id}-app`}
          value={selectValue}
          onChange={(event) => choose(event.target.value)}
          className="w-64 rounded border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-blue-500 dark:border-neutral-700 dark:bg-neutral-900"
        >
          <option value="">Not configured</option>
          {detected.map((app) => (
            <option key={app.id} value={app.path}>
              {app.name}
            </option>
          ))}
          <option value={CUSTOM}>Custom executable…</option>
        </select>
      </SettingRow>

      <SettingRow
        title="Executable path"
        description="Choose a program you trust. Knitt runs it directly, never through a shell."
        htmlFor={`${id}-exe`}
      >
        <div className="flex items-center gap-2">
          <TextInput
            id={`${id}-exe`}
            value={executable}
            onChange={(event) => setExecutable(event.target.value)}
            onBlur={() => applyExecutable(executable)}
            onKeyDown={(event) => {
              if (event.key === "Enter") applyExecutable(executable);
            }}
            placeholder="/usr/bin/…"
            className="w-64"
            spellCheck={false}
          />
          <button type="button" onClick={browse} className={textButton}>
            Browse…
          </button>
        </div>
      </SettingRow>

      <SettingRow
        title="Arguments"
        description="Use %TARGET_PATH% where the folder should go. Quotes group arguments; there is no shell expansion."
        htmlFor={`${id}-args`}
      >
        <TextInput
          id={`${id}-args`}
          value={args}
          onChange={(event) => setArgs(event.target.value)}
          onBlur={commitArguments}
          onKeyDown={(event) => {
            if (event.key === "Enter") commitArguments();
          }}
          placeholder={defaultArguments}
          className="w-80 font-mono text-xs"
          spellCheck={false}
        />
      </SettingRow>

      <div className="flex flex-wrap items-center gap-3 py-4">
        <button
          type="button"
          onClick={runTest}
          disabled={busy}
          className={primaryButton}
        >
          {busy ? "Starting…" : `Test ${kind}`}
        </button>
        {config.executable && target && (
          <button type="button" onClick={openSaved} disabled={busy} className={textButton}>
            Open current repository
          </button>
        )}
        {status && (
          <span
            role="status"
            className={`text-xs ${
              status.kind === "ok"
                ? "text-green-700 dark:text-green-400"
                : "text-red-700 dark:text-red-400"
            }`}
          >
            {status.text}
          </span>
        )}
      </div>

      {resolved && (
        <p className="truncate text-xs opacity-60" title={resolved}>
          Runs {resolved}
        </p>
      )}
    </>
  );
}
