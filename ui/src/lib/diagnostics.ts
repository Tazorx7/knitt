import { invoke } from "@tauri-apps/api/core";

/** Mirrors `config::diagnostics::EnvironmentInfo`. Non-secret by design. */
export type EnvironmentInfo = {
  app_version: string;
  git_backend: string;
  os: string;
  arch: string;
  desktop: string | null;
  settings_dir: string;
  data_dir: string;
  logs_dir: string;
  log_size: number;
  app_data_size: number;
  account_count: number;
};

export function loadEnvironment(): Promise<EnvironmentInfo> {
  return invoke<EnvironmentInfo>("diagnostics_environment");
}

/** The redacted report text, for review before saving. */
export function loadReport(): Promise<string> {
  return invoke<string>("diagnostics_report");
}

export function exportReport(path: string): Promise<void> {
  return invoke<void>("diagnostics_export", { path });
}

export function openLogsFolder(): Promise<void> {
  return invoke<void>("diagnostics_open_logs");
}

export function clearLogs(): Promise<void> {
  return invoke<void>("diagnostics_clear_logs");
}

/** One saved panic report in the app data folder. */
export type CrashReport = {
  file: string;
  summary: string;
  time: number;
  size: number;
};

export function loadCrashReports(): Promise<CrashReport[]> {
  return invoke<CrashReport[]>("crashes_list");
}

export function readCrashReport(file: string): Promise<string> {
  return invoke<string>("crashes_read", { file });
}

export function exportCrashReport(path: string, file: string): Promise<void> {
  return invoke<void>("crashes_export", { path, file });
}

export function clearCrashReports(): Promise<void> {
  return invoke<void>("crashes_clear");
}

/** Read-only SSH environment facts. Never includes key contents or paths. */
export type SshDiagnostics = {
  ssh_executable: boolean;
  agent_available: boolean;
  public_keys: string[];
  repository_checked: boolean;
  ssh_remotes: number;
};

export function loadSshDiagnostics(
  repoPath: string | null,
): Promise<SshDiagnostics> {
  return invoke<SshDiagnostics>("settings_ssh_diagnostics", {
    repoPath: repoPath ?? null,
  });
}
