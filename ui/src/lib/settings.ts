import { invoke } from "@tauri-apps/api/core";

export type ThemeMode = "light" | "dark" | "system";

/** Mirrors `config::settings::Settings` in the Rust backend. */
export type Settings = {
  schema_version: number;
  theme: ThemeMode;
  default_branch: string;
  restore_last_repository: boolean;
  default_repository_directory: string | null;
  last_repository_path: string | null;
  diff_tab_width: number;
};

export const settingsDefaults: Settings = {
  schema_version: 2,
  theme: "system",
  default_branch: "main",
  restore_last_repository: true,
  default_repository_directory: null,
  last_repository_path: null,
  diff_tab_width: 4,
};

export type GitIdentity = {
  name: string | null;
  email: string | null;
  config_path: string | null;
};

export function loadSettings(): Promise<Settings> {
  return invoke<Settings>("settings_get");
}

/** Persists and returns the backend-normalized settings. */
export function saveSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("settings_save", { settings });
}

export function loadGitIdentity(): Promise<GitIdentity> {
  return invoke<GitIdentity>("settings_git_identity");
}

/**
 * Passing `null` leaves a key untouched, so an empty form field can never
 * erase an existing global value.
 */
export function saveGitIdentity(
  name: string | null,
  email: string | null,
): Promise<GitIdentity> {
  return invoke<GitIdentity>("settings_set_git_identity", { name, email });
}
