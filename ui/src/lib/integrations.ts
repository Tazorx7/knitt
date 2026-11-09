import { invoke } from "@tauri-apps/api/core";

export type DetectedApp = {
  id: string;
  name: string;
  /** Resolved absolute path. */
  path: string;
  /** Default argument template, including `%TARGET_PATH%` where useful. */
  arguments: string;
};

export type DetectedIntegrations = {
  editors: DetectedApp[];
  terminals: DetectedApp[];
};

export function detectIntegrations(): Promise<DetectedIntegrations> {
  return invoke<DetectedIntegrations>("integrations_detect");
}

export function validateExecutable(executable: string): Promise<string> {
  return invoke<string>("integrations_validate", { executable });
}

export function testIntegration(
  kind: "editor" | "terminal",
  executable: string,
  args: string,
  target: string,
): Promise<void> {
  return invoke("integrations_test", { kind, executable, arguments: args, target });
}

export function openIntegration(
  kind: "editor" | "terminal",
  target: string,
): Promise<void> {
  return invoke("integrations_open", { kind, target });
}

export function openInFileManager(target: string): Promise<void> {
  return invoke("integrations_open_file_manager", { target });
}

export function defaultTestTarget(): Promise<string> {
  return invoke<string>("integrations_default_target");
}
