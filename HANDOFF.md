# knitt — Handoff Document

> **Date:** 2026-11-05
> **Repo:** `Tazorx7/knitt` (public)
> **Status:** MVP foundations, core git, GitHub auth/API, all side panels (history/tags/stash/remotes/staging) and the Monaco diff viewer working; advanced git (merge/rebase/conflicts/reflog/cherry-pick) still on unmerged feature branches; CI fully green.

---

## 1. Project Overview

**knitt** is a native Linux Git client built with **Tauri v2** (Rust backend + WebKitGTK webview) and a **React 19 + TypeScript** frontend. It is a clean-room reimplementation of the GitHub Desktop *workflow* (not its code or assets), with original branding.

| Aspect | Value |
|--------|-------|
| App name | `knitt` |
| Bundle identifier | `app.nativegitclient.desktop` |
| Version | `0.1.0` |
| Backend | Rust, Tauri v2.12.5, `git2`, `octocrab`, `keyring`, `notify`, `tracing`, `reqwest` |
| Frontend | React 19, TypeScript, Tailwind CSS, Radix UI, Zustand, Monaco |
| Auth | OAuth Device Flow (RFC 8628), tokens in Linux Secret Service via `keyring` |
| CI/CD | GitHub Actions (`ci.yml`, `blue-green.yml`, `feature.yml`) |
| License | Apache 2.0 |

---

## 2. What Is Done (merged into `main`/`master`)

### 2.1 Foundations (Phase 0)
- [x] Tauri v2 scaffolding (Rust backend + React/TS frontend)
- [x] App settings & config system (`config/settings.rs`, XDG paths via `config/paths.rs`)
- [x] Logging/diagnostics (`tracing`, `utils/logging.rs`)
- [x] Secure secret storage (`auth/secret_store.rs` via `keyring` — Linux Secret Service)
- [x] Strict Tauri IPC allowlist (`capabilities/default.json`: `core:default`, `dialog:default`, `shell:default`, `fs:default`)
- [x] File watching scaffold (`git/watcher.rs` via `notify`)

### 2.2 Core Git (Phase 1 — partial)
- [x] Local repo open (`repo_open` → `git/repo.rs::open_repo`)
- [x] Status (`repo_status` → `git/repo.rs::get_status`)
- [x] Commit (`repo_commit` → `git/commit.rs::commit_all`)
- [x] Push (`repo_push` → libgit2 + credential callbacks)
- [x] Branch list/create/checkout (`repo_branches`, `repo_create_branch`, `repo_checkout_branch` → `git/branch.rs`)
- [x] Per-file staging/unstaging (`repo_stage_files`, `repo_unstage_files`, `repo_stage_all`) with a staged/unstaged split in the Changes panel; commits now include only staged files
- [ ] Line- and hunk-level staging (`repo_stage_hunk`/`repo_unstage_hunk` still act on whole files and are not exposed)
- [x] Stage file/all (`repo_stage_files`, `repo_stage_all` → `git/staging.rs`)
- [x] Fetch/pull (`repo_fetch`, `repo_pull` → `git/remotes.rs`)
- [x] History (`repo_log` → `git/history.rs::log_commits`)
- [x] Stash (`repo_stash_list`/`save`/`pop`/`drop` → `git/stash.rs`)
- [x] Remote management (`repo_list_remotes`, `repo_add_remote`, `repo_remove_remote`, `repo_set_remote_url` → `git/remotes.rs`)
- [x] Tags (`repo_list_tags`, `repo_create_tag`, `repo_delete_tag`, `repo_push_tag` → `git/tags.rs`)
- [x] Diff content for the viewer (`repo_file_contents` → `git/status.rs::get_file_contents`, HEAD vs working tree)
- [x] Create a new repository (`repo_init` → `git/repo_init.rs`), from the New repository dialog, using the configured default branch as the initial branch
- [x] Clone a repository (`repo_clone` → `git/clone.rs`), with HTTPS, SSH or `file://`, progress events, and no credential storage

### 2.3 GitHub Integration (Phase 3 — partial)
- [x] OAuth Device Flow for github.com + GHES (`auth/device_flow.rs`, `commands/auth.rs`)
- [x] Multi-account storage + active account switching (`auth/accounts.rs`, `auth_list_accounts`, `auth_set_active_account`)
- [x] PR list/create/checkout (`gh_list_prs`, `gh_create_pr`, `gh_checkout_pr`)
- [x] Issues list (`gh_list_issues`)
- [x] Actions run list (`gh_list_workflow_runs`)
- [x] High-level repo-scoped commands that resolve owner/repo + token server-side:
  - `gh_list_repo_issues(path)`, `gh_list_repo_prs(path)`, `gh_list_repo_workflow_runs(path)`
  - Owner/repo parsed from the `origin` remote URL (`git/remotes.rs::get_github_owner_repo`), token resolved from active account (`auth/accounts.rs::active_token`) — **token never touches the frontend**
- [x] Fork sync (`repo_sync_fork` → `git/remotes.rs::sync_fork`)

### 2.4 UI (Phase 4 — partial)
- [x] Collapsible side panel layout (Changes, Branches, History, Remotes, Stash, Tags + Issues/PRs/Actions when authenticated)
- [x] Theme toggle (light/dark), dark/light logo assets
- [x] Keyboard shortcuts (`hooks/useKeyboardShortcuts.ts`)
- [x] Accessibility pass (ARIA labels, `aria-expanded`, focus-visible rings)
- [x] GitHub OAuth client ID is baked in at build time from `.env` / CI secret (no per-user setting)
- [x] Error sanitization — raw errors logged to console only, user sees friendly messages
- [x] Monaco side-by-side diff viewer (`components/DiffViewer.tsx`) — click a changed file in Changes; Monaco is bundled locally (offline, no CDN) and lazy-loaded so the initial bundle stays ~254 kB
- [x] Settings surface (`components/settings/`): General, Accounts, Integrations, Git and Appearance sections; gear in the header, `Ctrl+,`, Escape to close
- [x] Accounts section: connected accounts, active badge, use/sign-out, GitHub.com and Enterprise sign-in
- [x] Settings search matches names, descriptions and synonyms, then jumps to the control
- [x] Integrations section: detects editors/terminals from `PATH`, custom executable + argument template, test launches, file manager; direct argv, never a shell
- [x] Theme (light/dark/system) and diff tab width persist in the backend and apply immediately; diff viewer follows the app theme
- [x] Global Git identity read/write (`git/identity.rs`) — only changed keys are written, blank/unchanged values never erase existing config
- [x] Repository-specific identity — local config override with reset to global; global config is never touched
- [x] Advanced section: version/backend/platform, bounded log file with open/clear, opt-in redacted report, app-data size and confirmed settings reset
- [x] About section: logo, version, build channel, libgit2 version, source/issue/licence links, third-party notices
- [x] Accessibility section: shortcut toggle + reference table, reduced-motion override that follows the desktop preference
- [x] Sync strategies: pull (merge/rebase/ff-only), push (current/tracked branch), fetch pruning — real libgit2 operations with upstream tracking
- [x] Diff display settings: tab width, font size, split/unified, whitespace filter, line wrapping
- [x] Accent colour presets and comfortable/compact density
- [x] General startup view, launch at login (autostart desktop entry), and recent repositories with remove/forget
- [x] Safety section: confirmations for per-file discard, tag delete and stash drop, plus a real per-file Discard action in the Changes list
- [x] Network proxy: system/none/manual applied to fetch, pull, push, tag pushes and fork sync
- [x] SSH diagnostics: agent, public key names, ssh-on-PATH and per-repository SSH remotes, read-only
- [x] Commit signing: off/GPG/SSH via the user's gpg or ssh-keygen, signed through libgit2's commit buffer
- [x] Branch comparison panel: pick two local branches, see ahead/behind counts and changed files, open per-file diffs
- [x] Commit viewer: click a History row to see the commit message, author and changed files, then open a per-file diff
- [ ] Remaining work: none scheduled from setting.md; open items are listed below
- [ ] Virtualized history — not done

### 2.5 Packaging / CI (Phase 4)
- [x] `ci.yml` — `build-and-test` (frontend build, `cargo check`, `cargo clippy -D warnings`, `cargo test`) + `build-appimage` (AppImage/.deb/.rpm via `tauri-action`)
- [x] `feature.yml` — feature-branch validation
- [x] `blue-green.yml` — blue-green deploy slots
- [x] `.deb` + `.rpm` + `.AppImage` bundles build successfully and upload to a draft release
- [x] `workflow_dispatch` trigger added to `ci.yml`
- [ ] Flathub / AUR — not done
- [ ] Auto-updater with signatures — **`src-tauri/src/updater.rs` is empty (0 bytes)**
- [ ] Crash reporting (opt-in) — **`crash.rs` was removed** (stub deleted)

### 2.6 Tests
- [x] Unit tests for remote URL parsing (`git/remotes.rs` — 6 tests, HTTPS/SSH/scp/enterprise/error cases)
- [x] Integration tests against real temp repos (`git/tests.rs` — `log_commits`, tag create/list/delete, stash save/list/pop, `get_file_contents`) → **10 tests**
- [x] Settings tests — schema defaults/migration/roundtrip, atomic write + permissions, identity write/no-op/blank-value rules, integration trimming — **37 tests total**
- [x] Integration tests — argument template splitting/substitution and executable resolution, including rejection of directories and non-executable files
- [x] Playwright E2E scaffold exists (installed; no committed specs)

---

## 3. Architecture

```
ui/src/                      React 19 + TS frontend
  App.tsx                    main layout, auth/repo wiring, sign-in modal, settings route, error handling, diff pane
  components/*.tsx           panel components, DiffViewer (Monaco), ThemeToggle
  components/settings/       SettingsView, controls, sections/{General,Accounts,Integrations,Git,Appearance}
  lib/monaco.ts              Monaco local worker/loader setup (offline, no CDN)
  lib/settings.ts            Settings types + command wrappers
  lib/integrations.ts        Editor/terminal detection and launch wrappers
  lib/theme.ts               theme resolution/applying
  hooks/useKeyboardShortcuts.ts

src-tauri/src/
  auth/                      device_flow, accounts, secret_store
  commands/                  auth.rs, repo.rs, github.rs, settings.rs, integrations.rs  (Tauri #[command] handlers)
  config/                    settings.rs (schema v3, atomic writes), paths.rs
  git/                       repo, commit, branch, remotes, staging, status, history, stash, tags,
                             credentials, identity, watcher, tests
  github/                    client.rs (octocrab wrapper)
  integrations.rs            app detection, argv templating, direct process launch
  state/                     app.rs (DeviceFlowState)
  utils/                     logging.rs, http.rs
  lib.rs                     plugin registration + invoke_handler allowlist
  updater.rs                 (empty — auto-updater not implemented)
```

**Diff flow:** clicking a changed file calls `repo_file_contents(path, file)` → `git/status.rs::get_file_contents` returns `{ original (HEAD), modified (worktree) }` → rendered by the lazy-loaded `DiffViewer` (Monaco side-by-side).

**Credential flow (HTTPS):** `git/credentials.rs` builds a libgit2 credential callback that loads the active account's token from Secret Service (by matching the remote host) and presents it as `x-access-token`.

**GitHub owner/repo resolution:** `git/remotes.rs::get_github_owner_repo` parses `origin` remote URLs (https, ssh, scp-like) into `(owner, repo)`.

---

## 4. Running the App (Runbook)

### Environment quirks specific to this dev machine
1. **GLib schema `antialiasing` crash** — Ubuntu 26.04's `gsettings-desktop-schemas 50.0` moved `antialiasing`/`hinting`/`rgba-order` into a `.deprecated` schema, but WebKitGTK still queries the main schema → fatal `GLib-GIO-ERROR` on startup.
   - **Workaround:** user-level schema override at `~/.local/share/glib-2.0/schemas`, exported as `GSETTINGS_SCHEMA_DIR`.
2. **Snap env-var pollution** — launching from the VS Code snap terminal injects `GTK_EXE_PREFIX`, `GTK_IM_MODULE_FILE`, `GTK_PATH` and snap entries in `XDG_DATA_DIRS`. WebKitGTK then loads the snap's glibc and fails with `libpthread.so.0: undefined symbol: __libc_pthread_init`, surfaced as `WebKit encountered an internal error`.
   - **Workaround:** do not inherit the VS Code environment. Start from `env -i` with only the desktop session variables plus the cargo/nvm paths (below). Unsetting a few GTK variables is not enough because `XDG_DATA_DIRS` still points into the snap.

### Launch (full native app)
```bash
cd /home/tazor/GitHub-linux
env -i HOME="$HOME" USER="$USER" LOGNAME="$USER" SHELL=/bin/bash LANG=C.UTF-8 \
  PATH="$HOME/.nvm/versions/node/v24.22.1/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
  DISPLAY=:0 WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 \
  DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/1000/bus" \
  XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.FFLDY3 XDG_SESSION_TYPE=wayland \
  GSETTINGS_SCHEMA_DIR="$HOME/.local/share/glib-2.0/schemas" \
  cargo tauri dev
```
Adjust `XAUTHORITY` to the file reported by `echo $XAUTHORITY` in the session.

### Frontend-only preview (no backend)
```bash
cd ui && npx vite --port 1420 --host 127.0.0.1
```

### Local build checks (run before pushing)
```bash
cd ui && npm run build                          # tsc + vite
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test --all-targets
```

### GitHub sign-in (client ID is build-time only)
End users never configure sign-in. The OAuth **Device Flow** needs only a client
ID (no secret, no redirect). It is baked into the binary at build time:

1. `cp .env.example .env` (`.env` is gitignored).
2. Put the OAuth app's Client ID in `.env` as `GITHUB_CLIENT_ID=Ov23li...`.
3. Ensure **Device flow** is enabled on the OAuth app settings page.

`src-tauri/build.rs` reads `.env` (repo root) or the `GITHUB_CLIENT_ID` env var
and emits `cargo:rustc-env`; the value is available to Rust via
`option_env!("GITHUB_CLIENT_ID")`. CI passes the `GITHUB_CLIENT_ID` repository
secret into both build jobs. If unset, the build still succeeds but sign-in
returns "GitHub sign-in is not configured in this build."

---

## 5. Known Gaps & Bugs

### 5.1 Command registration / wiring gaps — ✅ resolved
All frontend panels now resolve to registered commands. `repo_diff`, `repo_stage_hunk`, and `repo_unstage_hunk` are registered, and the previously missing backends were added:

| Frontend invokes | Backend |
|------------------|---------|
| `repo_log` | ✅ `git/history.rs` (merged) |
| `repo_list_tags`, `repo_create_tag`, `repo_delete_tag`, `repo_push_tag` | ✅ `git/tags.rs` (merged) |
| `repo_stash_list`, `repo_stash_save`, `repo_stash_pop`, `repo_stash_drop` | ✅ `git/stash.rs` (merged) |
| `repo_list_remotes`, `repo_add_remote` (+ remove/set-url) | ✅ registered |
| `repo_stage_all`, `repo_stage_files` | ✅ registered |
| `repo_fetch`, `repo_pull` | ✅ registered |
| `repo_diff`, `repo_stage_hunk`, `repo_unstage_hunk` | ✅ registered |
| `repo_file_contents` | ✅ new (Monaco diff source) |

> Note: `staging::unstage_files` exists but still has **no command**; the UI currently only stages, it does not unstage selected files.

### 5.2 Feature branches with unmerged work
The following branches still contain backend modules that are **not merged into `main`**. (History, stash, tags, remotes, staging, hunk-staging, fork-sync, GitHub API/issues/PRs/actions and the Monaco diff viewer have since been merged and their branches can be deleted.)

| Branch | Contains (still unmerged) |
|--------|---------------------------|
| `feat/reflog` | `git/reflog.rs`, `git/merge.rs` |
| `feat/merge-rebase` | `git/merge.rs` (merge/rebase flows) |
| `feat/rebase` | rebase |
| `feat/conflicts` | `git/conflicts.rs` — conflict resolution |
| `feat/cherry-revert` | cherry-pick + revert |
| `feat/tags-advanced` | branch publish / upstream tracking (tag push/delete already merged) |
| `feat/upstream` | upstream tracking |
| `feat/pr-workflows` | PR workflows (repo-scoped PR list/create already merged) |
| `feat/monaco-diff`, `feat/diff-integration`, `feat/hunk-staging`, `feat/fork-sync`, `feat/github-api`, `feat/actions`, `feat/issues`, `feat/remotes`, `feat/stash`, `feat/history`, `feat/tags`, `feat/staging` | ✅ **already merged into `main`** — safe to delete |

> ⚠️ The still-unmerged branches were built on older `main` states and will likely need **rebasing / conflict resolution** before merging. Do not blindly merge.

### 5.3 Other gaps
- **Auto-updater** — `updater.rs` is empty; Tauri updater not configured (`tauri.conf.json` has no `updater` block).
- **Crash reporting** — stub was removed; opt-in crash reporting not implemented.
- **GitHub Enterprise API** — `octocrab` is hardcoded to the public `api.github.com`; GHES repos in Issues/PRs/Actions panels won't resolve.
- **Multi-line commit / amend / undo** — not implemented.
- **Line-level staging** — hunk-level only.
- **Virtualized history** — plain list; no TanStack Virtual.
- **External editor/mergetool/difftool** — not implemented.
- **Git LFS** — not implemented.

---

## 6. What Is Left (TODO — from `PRD.md`)

### ✅ Recently completed (was the "immediate / high-value" list)
1. ~~Merge/rebuild the unmerged git modules (stash, tags, history, remotes, staging)~~ — merged into `main`.
2. ~~Register the missing commands in `lib.rs`~~ — all panels wired (see §5.1).
3. ~~Wire the Monaco diff viewer~~ — done (`components/DiffViewer.tsx`, `repo_file_contents`).

### Immediate / high-value (next)
1. **Merge the advanced-git branches** (merge/rebase, conflicts, reflog, cherry-pick/revert) with rebasing — these bring conflict resolution UI.
2. **Add `repo_unstage_files` command** and wire unstaging in `StagingPanel` (backend fn already exists).
3. **Branch publish / upstream tracking** (`feat/tags-advanced`, `feat/upstream`).
4. **GHES API base URL** so Enterprise repos resolve in Issues/PRs/Actions panels.

### Phase 1 remaining (Core Git)
- [x] Clone (HTTPS/SSH) — `repo_clone` with account token or agent key; progress is streamed to the Clone dialog
- [x] Multi-line commit messages, amend the last commit and undo the last commit (soft reset keeps the changes staged); co-author trailers still pending
- [x] Branch rename, delete (merged only, never the current branch) and publish with upstream tracking, from the Branches panel
- [x] History view + diff viewer (unified/side-by-side Monaco) — done; virtualization still pending
- [x] Stash list/save/pop/drop (auto-prompt on switch still pending)
- [x] Remote management (add/remove/set-url), fetch/pull

### Phase 2 remaining (Advanced Git + Conflicts)
- [ ] Merge/rebase + conflict resolution UI
- [ ] Interactive rebase, cherry-pick, revert
- [x] Reflog panel: recent HEAD movements with short id and message; a row opens the commit viewer, so amended or undone commits remain reachable
- [x] Tags (list/create/push/delete)
- [ ] External editor/mergetool/difftool integration
- [ ] Git LFS (basic)

### Phase 3 remaining (GitHub)
- [ ] GHES (Enterprise) API base URL support
- [x] PR list UI (repo-scoped, token resolved server-side)
- [ ] Actions job log viewing
- [x] Issues view UI (read-only)

### Phase 4 remaining (Polish & Distribution)
- [ ] Auto-updater (Tauri updater) with signatures
- [ ] Crash reporting (opt-in, privacy-first)
- [ ] Flathub, AUR packaging
- [ ] Performance profiling against NFRs

### NFR validation (from §5/§7)
- [ ] Cold start < 400ms
- [ ] Idle RAM < 50MB
- [ ] Packaged binary < 15MB (current AppImage is 87 MB — webkit bundling; needs size investigation)
- [ ] Install/run on Ubuntu 24.04, Fedora 42, Arch (Wayland + X11)

---

## 7. Git / Branching Conventions

- Feature/fix work goes on `feat/*` / `fix/*` branches.
- `main`/`master` are **protected** (blue-green flow); never push directly, never force-push.
- Merge feature branches with `--no-ff`, sync `main` → `master`, delete the feature branch after merge.
- Run `cargo clippy --all-targets -- -D warnings` + `cd ui && npm run build` **before** pushing.

---

## 8. Immediate Next Actions (suggested order)

1. Merge the advanced-git branches (merge/rebase, conflicts, reflog, cherry-pick/revert), rebasing as needed.
2. Add `repo_unstage_files` + wire unstaging in `StagingPanel`; add branch publish/upstream tracking.
3. Implement auto-updater + crash reporting (Phase 4).
4. Investigate AppImage size vs the < 15 MB NFR target (Monaco adds a lazy chunk too).
5. Profile cold start / RAM / UI performance against NFRs.
6. Create the OAuth app (device flow enabled) and set `GITHUB_CLIENT_ID` in `.env` + the CI secret.
7. Clean up stale merged remote branches (`feat/stash`, `feat/tags`, `feat/history`, `feat/monaco-diff`, …).
