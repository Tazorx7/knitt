import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  SectionHeader,
  SettingRow,
  TextInput,
  type SectionProps,
} from "../controls";
import { loadGitIdentity, saveGitIdentity, type GitIdentity } from "../../../lib/settings";

type Status = { kind: "ok" | "error"; text: string } | null;

export default function GitSection({ settings, onChange }: SectionProps) {
  const [identity, setIdentity] = useState<GitIdentity | null>(null);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<Status>(null);

  const [branch, setBranch] = useState(settings.default_branch);
  const [branchError, setBranchError] = useState<string | null>(null);

  useEffect(() => {
    setBranch(settings.default_branch);
  }, [settings.default_branch]);

  useEffect(() => {
    loadGitIdentity()
      .then((loaded) => {
        setIdentity(loaded);
        setName(loaded.name ?? "");
        setEmail(loaded.email ?? "");
      })
      .catch(() => setStatus({ kind: "error", text: "Could not read your Git identity." }));
  }, []);

  async function saveIdentity() {
    setSaving(true);
    setStatus(null);
    try {
      const saved = await saveGitIdentity(
        name.trim() === "" ? null : name.trim(),
        email.trim() === "" ? null : email.trim(),
      );
      setIdentity(saved);
      setStatus({ kind: "ok", text: "Saved to your global Git configuration." });
    } catch (error) {
      setStatus({
        kind: "error",
        text: typeof error === "string" ? error : "Could not save your Git identity.",
      });
    } finally {
      setSaving(false);
    }
  }

  async function commitBranch() {
    const value = branch.trim();
    if (value === settings.default_branch) {
      setBranchError(null);
      return;
    }
    try {
      await invoke("settings_validate_branch", { name: value });
      setBranchError(null);
      onChange({ default_branch: value });
    } catch {
      setBranchError("Enter a valid branch name, for example main or release/2.0.");
    }
  }

  return (
    <>
      <SectionHeader
        title="Git"
        description="Commit identity and repository defaults. Knitt never edits other Git configuration."
      />

      <h2 className="text-sm font-semibold">Commit identity</h2>
      <p className="mt-1 text-xs opacity-70">
        Used as the author for commits unless a repository overrides it. Commit
        author names and emails are public metadata once commits are pushed.
      </p>

      <SettingRow title="Name" description="Your Git author name." htmlFor="git-name">
        <TextInput
          id="git-name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="Your name"
          className="w-64"
          autoComplete="name"
        />
      </SettingRow>

      <SettingRow title="Email" description="Your Git author email." htmlFor="git-email">
        <TextInput
          id="git-email"
          type="email"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          placeholder="you@example.com"
          className="w-64"
          autoComplete="email"
        />
      </SettingRow>

      <div className="flex items-center gap-3 py-4">
        <button
          type="button"
          onClick={saveIdentity}
          disabled={saving}
          className="rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 disabled:opacity-50 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
        >
          {saving ? "Saving…" : "Save identity"}
        </button>
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

      <p className="text-xs opacity-60">
        {identity?.config_path
          ? `Writes to ${identity.config_path}. Only changed fields are written.`
          : "Knitt could not locate a global Git configuration file."}
      </p>

      <div className="mt-8">
        <h2 className="text-sm font-semibold">Defaults</h2>
      </div>

      <SettingRow
        title="Default branch name"
        description="Used when Knitt creates a new repository. Creating repositories is not available yet, so this is stored for that flow."
        htmlFor="default-branch"
      >
        <div className="flex flex-col items-end gap-1">
          <TextInput
            id="default-branch"
            value={branch}
            onChange={(event) => setBranch(event.target.value)}
            onBlur={commitBranch}
            onKeyDown={(event) => {
              if (event.key === "Enter") commitBranch();
            }}
            placeholder="main"
            className="w-48"
            aria-invalid={branchError ? true : undefined}
            aria-describedby={branchError ? "default-branch-error" : undefined}
          />
          {branchError && (
            <span id="default-branch-error" className="text-xs text-red-700 dark:text-red-400">
              {branchError}
            </span>
          )}
        </div>
      </SettingRow>
    </>
  );
}
