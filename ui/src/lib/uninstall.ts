import { invoke } from "@tauri-apps/api/core";

/** Mirrors `system::uninstall::InstallKind`. */
export type InstallKind = "deb" | "rpm" | "appimage" | "source";

export type RemovalItem = {
  path: string;
  description: string;
};

/** Mirrors `system::uninstall::UninstallPlan`. */
export type UninstallPlan = {
  kind: InstallKind;
  kind_label: string;
  package: string | null;
  needs_password: boolean;
  system_items: string[];
  data_items: RemovalItem[];
  keyring_accounts: number;
};

/** Mirrors `system::uninstall::UninstallOutcome`. */
export type UninstallOutcome = {
  /** sudo rejected the password (or needs one that was not supplied). */
  auth_failed: boolean;
  system_removed: boolean;
  data_removed: string[];
  failures: string[];
};

export function loadUninstallPlan(): Promise<UninstallPlan> {
  return invoke<UninstallPlan>("app_uninstall_plan");
}

export function uninstallKnitt(password?: string): Promise<UninstallOutcome> {
  return invoke<UninstallOutcome>("app_uninstall", { password: password ?? null });
}

export function quitApp(): Promise<void> {
  return invoke<void>("app_quit");
}
