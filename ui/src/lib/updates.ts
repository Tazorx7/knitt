import { invoke } from "@tauri-apps/api/core";

/** Mirrors `commands::updates::UpdateStatus`. */
export type UpdateStatus = {
  current_version: string;
  /** True only when a newer version exists and this build can install it. */
  available: boolean;
  version: string | null;
  notes: string | null;
  /** Set when this build cannot update itself at all (deb, rpm, source). */
  unavailable: string | null;
};

/** Mirrors `commands::updates::UpdateProgress`, emitted while downloading. */
export type UpdateProgress = {
  downloaded: number;
  total: number | null;
};

export function checkForUpdates() {
  return invoke<UpdateStatus>("updates_check");
}

/**
 * Downloads and applies the update. On success the backend restarts the app,
 * so this promise does not resolve in the normal case.
 */
export function installUpdate() {
  return invoke<void>("updates_install");
}
