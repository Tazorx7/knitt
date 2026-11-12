import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import {
  SectionHeader,
  SettingRow,
  Segmented,
  TextInput,
  type SectionProps,
} from "../controls";
import { proxyUrlProblem, type ProxyMode } from "../../../lib/settings";
import {
  clearLogs,
  exportReport,
  loadEnvironment,
  loadReport,
  openLogsFolder,
  type EnvironmentInfo,
} from "../../../lib/diagnostics";

type Status = { kind: "ok" | "error"; text: string } | null;

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
}

export default function AdvancedSection({
  settings,
  onChange,
  onReset,
}: SectionProps & { onReset: () => Promise<void> }) {
  const [env, setEnv] = useState<EnvironmentInfo | null>(null);
  const [proxyUrl, setProxyUrl] = useState(settings.proxy_url);
  const [proxyProblem, setProxyProblem] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>(null);
  const [report, setReport] = useState<string | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);
  const [busy, setBusy] = useState(false);

  function refresh() {
    loadEnvironment()
      .then(setEnv)
      .catch(() =>
        setStatus({ kind: "error", text: "Could not read the environment details." }),
      );
  }

  useEffect(refresh, []);

  useEffect(() => {
    setProxyUrl(settings.proxy_url);
    setProxyProblem(null);
  }, [settings.proxy_url]);

  function commitProxyUrl() {
    const value = proxyUrl.trim();
    const problem = proxyUrlProblem(value);
    setProxyProblem(problem);
    if (problem) return;
    if (value !== settings.proxy_url) {
      onChange({ proxy_url: value });
    }
  }

  function changeProxyMode(mode: ProxyMode) {
    setProxyProblem(null);
    onChange({ proxy_mode: mode });
  }

  async function handleOpenLogs() {
    setStatus(null);
    try {
      await openLogsFolder();
    } catch {
      setStatus({ kind: "error", text: "Could not open the log folder." });
    }
  }

  async function handleClearLogs() {
    setBusy(true);
    setStatus(null);
    try {
      await clearLogs();
      setStatus({ kind: "ok", text: "Logs cleared." });
      refresh();
    } catch {
      setStatus({ kind: "error", text: "Could not clear the logs." });
    } finally {
      setBusy(false);
    }
  }

  async function handleReviewReport() {
    setStatus(null);
    try {
      setReport(await loadReport());
    } catch {
      setStatus({ kind: "error", text: "Could not build the diagnostic report." });
    }
  }

  async function handleExportReport() {
    setBusy(true);
    setStatus(null);
    try {
      const path = await save({
        title: "Save diagnostic report",
        defaultPath: "knitt-diagnostics.txt",
      });
      if (typeof path !== "string") return;
      await exportReport(path);
      setStatus({ kind: "ok", text: "Diagnostic report saved." });
    } catch {
      setStatus({ kind: "error", text: "Could not save the diagnostic report." });
    } finally {
      setBusy(false);
    }
  }

  async function handleReset() {
    setBusy(true);
    setStatus(null);
    try {
      await onReset();
      setConfirmReset(false);
      setStatus({ kind: "ok", text: "Settings reset to defaults." });
      refresh();
    } catch {
      setStatus({ kind: "error", text: "Could not reset settings." });
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <SectionHeader
        title="Advanced"
        description="Diagnostics, logs and application data. Nothing here changes your repositories."
      />

      <h2 className="text-sm font-semibold">Network</h2>
      <p className="mt-1 text-xs opacity-70">
        Applies to fetch, pull, push and tag pushes. Credentials are never
        stored here.
      </p>

      <SettingRow
        id="setting-proxy-mode"
        title="Proxy"
        description="System reuses the proxy configured in Git or the environment. Manual connects directly until you enter an address."
      >
        <Segmented<ProxyMode>
          value={settings.proxy_mode}
          label="Proxy mode"
          options={[
            { value: "system", label: "System" },
            { value: "none", label: "None" },
            { value: "manual", label: "Manual" },
          ]}
          onChange={changeProxyMode}
        />
      </SettingRow>

      {settings.proxy_mode === "manual" && (
        <SettingRow
          id="setting-proxy-url"
          title="Proxy address"
          description="Example: http://proxy.example:3128 or socks5://127.0.0.1:9050. Usernames and passwords are rejected."
          htmlFor="proxy-url"
        >
          <TextInput
            id="proxy-url"
            type="text"
            value={proxyUrl}
            placeholder="http://proxy.example:3128"
            aria-invalid={proxyProblem ? true : undefined}
            aria-describedby={proxyProblem ? "proxy-url-problem" : undefined}
            onChange={(event) => setProxyUrl(event.target.value)}
            onBlur={commitProxyUrl}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                commitProxyUrl();
              }
            }}
            className="w-72"
          />
        </SettingRow>
      )}

      {proxyProblem && (
        <p
          id="proxy-url-problem"
          role="status"
          className="pb-3 text-xs text-red-700 dark:text-red-400"
        >
          {proxyProblem}
        </p>
      )}

      <h2 className="mt-6 text-sm font-semibold">Diagnostics</h2>

      <SettingRow
        id="setting-advanced-version"
        title="Application"
        description="Build details for bug reports."
      >
        <span className="text-xs opacity-80">
          {env ? `Knitt ${env.app_version}` : "Loading…"}
        </span>
      </SettingRow>

      <SettingRow
        title="Git backend"
        description="Knitt uses an embedded Git library, not a system git binary."
      >
        <span className="text-xs opacity-80">{env?.git_backend ?? "Loading…"}</span>
      </SettingRow>

      <SettingRow title="Platform" description="Operating system, architecture and desktop.">
        <span className="text-xs opacity-80">
          {env
            ? `${env.os} ${env.arch}${env.desktop ? ` · ${env.desktop}` : ""}`
            : "Loading…"}
        </span>
      </SettingRow>

      <SettingRow
        id="setting-advanced-logs"
        title="Logs"
        description={
          env
            ? `${formatBytes(env.log_size)} in ${env.logs_dir}. Logs rotate automatically and never include tokens.`
            : "Where Knitt writes its diagnostic log."
        }
      >
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={handleOpenLogs}
            className="rounded border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Open logs folder
          </button>
          <button
            type="button"
            onClick={handleClearLogs}
            disabled={busy}
            className="rounded px-2 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Clear logs
          </button>
        </div>
      </SettingRow>

      <SettingRow
        id="setting-advanced-report"
        title="Diagnostic report"
        description="Review a redacted report before saving it. It contains no tokens, repository paths or file contents."
      >
        <div className="flex flex-wrap justify-end gap-2">
          <button
            type="button"
            onClick={handleReviewReport}
            className="rounded border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Review report
          </button>
          <button
            type="button"
            onClick={handleExportReport}
            disabled={busy}
            className="rounded px-2 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Save report…
          </button>
        </div>
      </SettingRow>

      {report !== null && (
        <div className="mb-4 mt-4 rounded border border-neutral-200 p-3 dark:border-neutral-800">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-xs font-semibold uppercase tracking-wide opacity-70">
              Report preview
            </h3>
            <button
              type="button"
              onClick={() => setReport(null)}
              className="rounded px-2 py-0.5 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Close
            </button>
          </div>
          <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words text-xs opacity-80">
            {report}
          </pre>
        </div>
      )}

      <h2 className="mt-8 text-sm font-semibold">Application data</h2>

      <SettingRow
        title="Storage"
        description="Settings, accounts metadata and logs. Repositories live wherever you cloned them."
      >
        <span className="text-xs opacity-80">
          {env ? formatBytes(env.app_data_size) : "Loading…"}
        </span>
      </SettingRow>

      <SettingRow title="Settings folder" description={env?.settings_dir ?? "Loading…"}>
        <span className="text-xs opacity-60">settings.json</span>
      </SettingRow>

      <SettingRow title="Logs folder" description={env?.logs_dir ?? "Loading…"}>
        <span className="text-xs opacity-60">knitt.log</span>
      </SettingRow>

      <SettingRow
        id="setting-advanced-reset"
        title="Reset settings"
        description="Restores defaults. Repositories, downloaded objects, accounts and saved credentials are kept."
      >
        {confirmReset ? (
          <div className="flex flex-wrap items-center justify-end gap-2">
            <span className="text-xs opacity-70">Reset all preferences?</span>
            <button
              type="button"
              onClick={handleReset}
              disabled={busy}
              className="rounded bg-red-600 px-3 py-1 text-sm text-white hover:bg-red-500 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              {busy ? "Resetting…" : "Reset"}
            </button>
            <button
              type="button"
              onClick={() => setConfirmReset(false)}
              disabled={busy}
              className="rounded px-2 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
            >
              Cancel
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={() => setConfirmReset(true)}
            className="rounded border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            Reset settings…
          </button>
        )}
      </SettingRow>

      {status && (
        <p
          role="status"
          className={`mt-3 text-xs ${
            status.kind === "ok"
              ? "text-green-700 dark:text-green-400"
              : "text-red-700 dark:text-red-400"
          }`}
        >
          {status.text}
        </p>
      )}
    </>
  );
}
