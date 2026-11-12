import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  SectionHeader,
  SettingRow,
  Segmented,
  TextInput,
  Toggle,
  type SectionProps,
} from "../controls";
import {
  loadGitIdentity,
  saveGitIdentity,
  loadRepositoryIdentity,
  saveRepositoryIdentity,
  resetRepositoryIdentity,
  type GitIdentity,
  type PullStrategy,
  type PushMode,
  type RepositoryIdentity,
} from "../../../lib/settings";

type Status = { kind: "ok" | "error"; text: string } | null;

export default function GitSection({
  settings,
  onChange,
  repoPath,
}: SectionProps & { repoPath: string | null }) {
  const [identity, setIdentity] = useState<GitIdentity | null>(null);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<Status>(null);

  const [branch, setBranch] = useState(settings.default_branch);
  const [branchError, setBranchError] = useState<string | null>(null);

  const [repoIdentity, setRepoIdentity] = useState<RepositoryIdentity | null>(null);
  const [repoMode, setRepoMode] = useState<"global" | "custom">("global");
  const [repoName, setRepoName] = useState("");
  const [repoEmail, setRepoEmail] = useState("");
  const [repoSaving, setRepoSaving] = useState(false);
  const [repoStatus, setRepoStatus] = useState<Status>(null);

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

  useEffect(() => {
    if (!repoPath) {
      setRepoIdentity(null);
      return;
    }
    let active = true;
    setRepoStatus(null);
    loadRepositoryIdentity(repoPath)
      .then((loaded) => {
        if (!active) return;
        setRepoIdentity(loaded);
        setRepoMode(loaded.uses_global ? "global" : "custom");
        setRepoName(loaded.local_name ?? loaded.name ?? "");
        setRepoEmail(loaded.local_email ?? loaded.email ?? "");
      })
      .catch(() => {
        if (active) {
          setRepoStatus({ kind: "error", text: "Could not read this repository's identity." });
        }
      });
    return () => {
      active = false;
    };
  }, [repoPath]);

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

  function startCustom() {
    setRepoName(repoIdentity?.name ?? "");
    setRepoEmail(repoIdentity?.email ?? "");
    setRepoMode("custom");
    setRepoStatus(null);
  }

  async function saveRepoIdentity() {
    if (!repoPath) return;
    const nextName = repoName.trim();
    const nextEmail = repoEmail.trim();
    if (!nextName) {
      setRepoStatus({ kind: "error", text: "Enter a name." });
      return;
    }
    if (!nextEmail || !nextEmail.includes("@") || /\s/.test(nextEmail)) {
      setRepoStatus({ kind: "error", text: "Enter a valid email address." });
      return;
    }
    setRepoSaving(true);
    setRepoStatus(null);
    try {
      const saved = await saveRepositoryIdentity(repoPath, nextName, nextEmail);
      setRepoIdentity(saved);
      setRepoName(saved.local_name ?? nextName);
      setRepoEmail(saved.local_email ?? nextEmail);
      setRepoMode("custom");
      setRepoStatus({ kind: "ok", text: "Saved for this repository only." });
    } catch (error) {
      setRepoStatus({
        kind: "error",
        text: typeof error === "string" ? error : "Could not save this repository's identity.",
      });
    } finally {
      setRepoSaving(false);
    }
  }

  async function resetRepoIdentity() {
    if (!repoPath) return;
    setRepoSaving(true);
    setRepoStatus(null);
    try {
      const saved = await resetRepositoryIdentity(repoPath);
      setRepoIdentity(saved);
      setRepoMode("global");
      setRepoName(saved.name ?? "");
      setRepoEmail(saved.email ?? "");
      setRepoStatus({ kind: "ok", text: "Using the global identity." });
    } catch (error) {
      setRepoStatus({
        kind: "error",
        text: typeof error === "string" ? error : "Could not reset this repository's identity.",
      });
    } finally {
      setRepoSaving(false);
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
        Clearing a field leaves the existing value unchanged.
      </p>

      <SettingRow id="setting-git-name" title="Name" description="Your Git author name." htmlFor="git-name">
        <TextInput
          id="git-name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="Your name"
          className="w-64"
          autoComplete="name"
        />
      </SettingRow>

      <SettingRow id="setting-git-email" title="Email" description="Your Git author email." htmlFor="git-email">
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
          className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
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
        <h2 className="text-sm font-semibold">This repository</h2>
        {!repoPath ? (
          <p className="mt-1 text-xs opacity-70">
            Open a repository to give it a different commit identity from your
            global one.
          </p>
        ) : (
          <>
            <p className="mt-1 text-xs opacity-70">
              Overrides the global identity for commits in this repository only.
              Commit author details become public metadata once pushed.
            </p>

            <SettingRow
              title="Effective identity"
              description={
                repoIdentity?.uses_global
                  ? "Inherited from global configuration."
                  : "Repository-specific."
              }
            >
              <span className="max-w-64 truncate text-xs opacity-80">
                {repoIdentity
                  ? `${repoIdentity.name ?? "No name set"} <${repoIdentity.email ?? "no email"}>`
                  : "Loading…"}
              </span>
            </SettingRow>

            {repoMode === "global" ? (
              <div className="py-4">
                <button
                  type="button"
                  onClick={startCustom}
                  className="rounded border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                >
                  Use a custom identity for this repository
                </button>
              </div>
            ) : (
              <>
                <SettingRow
                  id="setting-repo-git-name"
                  title="Name"
                  description="Author name for commits in this repository."
                  htmlFor="repo-git-name"
                >
                  <TextInput
                    id="repo-git-name"
                    value={repoName}
                    onChange={(event) => setRepoName(event.target.value)}
                    placeholder="Your name"
                    className="w-64"
                    autoComplete="name"
                  />
                </SettingRow>

                <SettingRow
                  id="setting-repo-git-email"
                  title="Email"
                  description="Author email for commits in this repository."
                  htmlFor="repo-git-email"
                >
                  <TextInput
                    id="repo-git-email"
                    type="email"
                    value={repoEmail}
                    onChange={(event) => setRepoEmail(event.target.value)}
                    placeholder="you@example.com"
                    className="w-64"
                    autoComplete="email"
                  />
                </SettingRow>

                <div className="flex flex-wrap items-center gap-3 py-4">
                  <button
                    type="button"
                    onClick={saveRepoIdentity}
                    disabled={repoSaving}
                    className="rounded bg-[var(--accent)] px-3 py-1 text-sm text-white hover:opacity-90 disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                  >
                    {repoSaving ? "Saving…" : "Save for this repository"}
                  </button>
                  {repoIdentity && !repoIdentity.uses_global ? (
                    <button
                      type="button"
                      onClick={resetRepoIdentity}
                      disabled={repoSaving}
                      className="rounded px-2 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                    >
                      Use global identity
                    </button>
                  ) : (
                    <button
                      type="button"
                      onClick={() => {
                        setRepoMode("global");
                        setRepoStatus(null);
                      }}
                      disabled={repoSaving}
                      className="rounded px-2 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                    >
                      Cancel
                    </button>
                  )}
                  {repoStatus && (
                    <span
                      role="status"
                      className={`text-xs ${
                        repoStatus.kind === "ok"
                          ? "text-green-700 dark:text-green-400"
                          : "text-red-700 dark:text-red-400"
                      }`}
                    >
                      {repoStatus.text}
                    </span>
                  )}
                </div>
              </>
            )}

            <p className="truncate text-xs opacity-60" title={repoPath}>
              {repoPath}
            </p>
          </>
        )}
      </div>

      <div className="mt-8">
        <h2 className="text-sm font-semibold">Synchronisation</h2>
        <p className="mt-1 text-xs opacity-70">
          How Knitt brings in remote changes and where it pushes.
        </p>
      </div>

      <SettingRow
        id="setting-pull-strategy"
        title="Pull strategy"
        description="Rebase keeps history linear; fast-forward only refuses to create merge commits or rewrite local commits."
      >
        <Segmented<PullStrategy>
          value={settings.pull_strategy}
          label="Pull strategy"
          options={[
            { value: "merge", label: "Merge" },
            { value: "rebase", label: "Rebase" },
            { value: "ff-only", label: "Fast-forward only" },
          ]}
          onChange={(pull_strategy) => onChange({ pull_strategy })}
        />
      </SettingRow>

      <SettingRow
        id="setting-push-mode"
        title="Push mode"
        description="Current pushes this branch to a same-named branch; upstream pushes to the branch this one already tracks."
      >
        <Segmented<PushMode>
          value={settings.push_mode}
          label="Push mode"
          options={[
            { value: "current", label: "Current branch" },
            { value: "upstream", label: "Tracked branch" },
          ]}
          onChange={(push_mode) => onChange({ push_mode })}
        />
      </SettingRow>

      <SettingRow
        id="setting-fetch-prune"
        title="Prune on fetch"
        description="Remove remote-tracking branches that no longer exist on the remote. Local branches are never deleted."
      >
        <Toggle
          checked={settings.fetch_prune}
          onChange={(fetch_prune) => onChange({ fetch_prune })}
          label="Prune on fetch"
        />
      </SettingRow>

      <div className="mt-8">
        <h2 className="text-sm font-semibold">Defaults</h2>
      </div>

      <SettingRow
        id="setting-default-branch"
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
