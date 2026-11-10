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
