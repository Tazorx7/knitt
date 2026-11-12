import { invoke } from "@tauri-apps/api/core";

export type ThemeMode = "light" | "dark" | "system";
export type PullStrategy = "merge" | "rebase" | "ff-only";
export type PushMode = "current" | "upstream";
export type Density = "comfortable" | "compact";
export type Accent = "blue" | "violet" | "emerald" | "amber" | "rose";
export type StartupView = "last" | "empty";

export type IntegrationConfig = {
  executable: string | null;
  arguments: string;
};

/** Mirrors `config::settings::Settings` in the Rust backend. */
export type Settings = {
  schema_version: number;
  theme: ThemeMode;
  default_branch: string;
  restore_last_repository: boolean;
  default_repository_directory: string | null;
  last_repository_path: string | null;
  recent_repositories: string[];
  startup_view: StartupView;
  autostart: boolean;
  diff_tab_width: number;
  diff_side_by_side: boolean;
  diff_ignore_whitespace: boolean;
  diff_wrap: boolean;
  diff_font_size: number;
  density: Density;
  accent: Accent;
  keyboard_shortcuts: boolean;
  reduce_motion: boolean;
  pull_strategy: PullStrategy;
  push_mode: PushMode;
  fetch_prune: boolean;
  external_editor: IntegrationConfig;
  terminal: IntegrationConfig;
};

export const settingsDefaults: Settings = {
  schema_version: 8,
  theme: "system",
  default_branch: "main",
  restore_last_repository: true,
  default_repository_directory: null,
  last_repository_path: null,
  recent_repositories: [],
  startup_view: "last",
  autostart: false,
  diff_tab_width: 4,
  diff_side_by_side: true,
  diff_ignore_whitespace: true,
  diff_wrap: true,
  diff_font_size: 12,
  density: "comfortable",
  accent: "blue",
  keyboard_shortcuts: true,
  reduce_motion: false,
  pull_strategy: "merge",
  push_mode: "current",
  fetch_prune: false,
  external_editor: { executable: null, arguments: "%TARGET_PATH%" },
  terminal: { executable: null, arguments: "--working-directory=%TARGET_PATH%" },
};

export type GitIdentity = {
  name: string | null;
  email: string | null;
  config_path: string | null;
};

export type RepositoryIdentity = {
  name: string | null;
  email: string | null;
  local_name: string | null;
  local_email: string | null;
  uses_global: boolean;
};

export function loadSettings(): Promise<Settings> {
  return invoke<Settings>("settings_get");
}

/** Persists and returns the backend-normalized settings. */
export function saveSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("settings_save", { settings });
}

/** Restores defaults. Accounts and saved credentials are preserved. */
export function resetSettings(): Promise<Settings> {
  return invoke<Settings>("settings_reset");
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

export function loadRepositoryIdentity(repoPath: string): Promise<RepositoryIdentity> {
  return invoke<RepositoryIdentity>("settings_repository_identity", { path: repoPath });
}

/** Writes the repository's own config only; never the global identity. */
export function saveRepositoryIdentity(
  repoPath: string,
  name: string,
  email: string,
): Promise<RepositoryIdentity> {
  return invoke<RepositoryIdentity>("settings_set_repository_identity", {
    path: repoPath,
    name,
    email,
  });
}

/** Removes local keys so the global identity is inherited again. */
export function resetRepositoryIdentity(repoPath: string): Promise<RepositoryIdentity> {
  return invoke<RepositoryIdentity>("settings_reset_repository_identity", {
    path: repoPath,
  });
}

/** Writes or removes the desktop autostart entry; returns its actual state. */
export function setAutostart(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("settings_set_autostart", { enabled });
}

/** Removes one entry from the recent list; never deletes files on disk. */
export function removeRecent(path: string): Promise<Settings> {
  return invoke<Settings>("settings_remove_recent", { path });
}

export function forgetRecent(): Promise<Settings> {
  return invoke<Settings>("settings_forget_recent");
}
