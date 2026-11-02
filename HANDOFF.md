# knitt — Handoff Document

> **Date:** 2026-11-02
> **Repo:** `Tazorx7/knitt` (public)
> **Status:** MVP foundations + core git + GitHub auth/API working; several advanced git features exist on unmerged feature branches; packaging pipeline green.

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
- [x] Hunk staging/unstaging + diff (`repo_stage_hunk`, `repo_unstage_hunk`, `repo_diff` — **defined but NOT registered**, see §5.1)
- [ ] Stage file/line granularity — `git/staging.rs` has `stage_files`/`unstage_files`/`stage_all`, but **no commands registered**
- [ ] Fetch/pull — `git/remotes.rs` has `fetch_remote`/`pull_remote`, but **no commands registered**
- [ ] History view — **no `git/history.rs` on main** (exists on `feat/history`)
- [ ] Stash — **no `git/stash.rs` on main** (exists on `feat/stash`)
- [ ] Remote management commands — `git/remotes.rs` has `list_remotes`/`add_remote`/`remove_remote`/`set_remote_url`, but **no commands registered**

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
- [x] Settings modal with GitHub OAuth client ID (`repo_get_settings`/`repo_save_settings`)
- [x] Error sanitization — raw errors logged to console only, user sees friendly messages
- [ ] Monaco diff viewer — not wired (exists on `feat/monaco-diff`)
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
- [x] Playwright E2E scaffold exists (installed; no committed specs)

---

## 3. Architecture

```
ui/src/                      React 19 + TS frontend
  App.tsx                    main layout, auth/repo wiring, settings modal, error handling
  components/*.tsx           panel components + ThemeToggle
  hooks/useKeyboardShortcuts.ts

src-tauri/src/
  auth/                      device_flow, accounts, secret_store
  commands/                  auth.rs, repo.rs, github.rs  (Tauri #[command] handlers)
  config/                    settings.rs, paths.rs
  git/                       repo, commit, branch, remotes, staging, status, credentials, watcher
  github/                    client.rs (octocrab wrapper)
  state/                     app.rs (DeviceFlowState)
  utils/                     logging.rs, http.rs
  lib.rs                     plugin registration + invoke_handler allowlist
  updater.rs                 (empty — auto-updater not implemented)
```

**Credential flow (HTTPS):** `git/credentials.rs` builds a libgit2 credential callback that loads the active account's token from Secret Service (by matching the remote host) and presents it as `x-access-token`.

**GitHub owner/repo resolution:** `git/remotes.rs::get_github_owner_repo` parses `origin` remote URLs (https, ssh, scp-like) into `(owner, repo)`.

---

## 4. Running the App (Runbook)

### Environment quirks specific to this dev machine
1. **GLib schema `antialiasing` crash** — Ubuntu 26.04's `gsettings-desktop-schemas 50.0` moved `antialiasing`/`hinting`/`rgba-order` into a `.deprecated` schema, but WebKitGTK still queries the main schema → fatal `GLib-GIO-ERROR` on startup.
   - **Workaround:** user-level schema override at `~/.local/share/glib-2.0/schemas`, exported as `GSETTINGS_SCHEMA_DIR`.
2. **Snap env-var pollution** — the shell leaks VSCode-snap vars (`GTK_PATH=/snap/code/...`, `LOCPATH`, etc.) that make GTK load the snap's glibc → `libpthread.so.0: undefined symbol`.
   - **Workaround:** `env -u GTK_PATH -u GTK_EXE_PREFIX -u GTK_IM_MODULE_FILE -u LOCPATH -u GIO_MODULE_DIR -u GTK_MODULES ...` before launch.

### Launch (full native app)
```bash
export PATH="$HOME/.cargo/bin:$PATH"
export GSETTINGS_SCHEMA_DIR="$HOME/.local/share/glib-2.0/schemas"
env -u GTK_PATH -u GTK_EXE_PREFIX -u LOCPATH -u GIO_MODULE_DIR -u GTK_MODULES \
  cargo tauri dev        # from repo root
```

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

### Sign-in requires a real OAuth client ID
Set it via the **⚙️ Settings** modal in-app (persisted to `settings.json`), or export `GITHUB_CLIENT_ID`. The placeholder `Ov23liPlaceholderClientId` intentionally returns a clear "not configured" error.

---

## 5. Known Gaps & Bugs

### 5.1 Command registration / wiring gaps (important)
The frontend panels reference commands that are **not registered** in `lib.rs` (or not implemented on `main`):

| Frontend invokes | Backend status |
|------------------|----------------|
| `repo_log` | ❌ no `git/history.rs` on main |
| `repo_list_tags`, `repo_create_tag`, `repo_delete_tag`, `repo_push_tag` | ❌ no `git/tags.rs` on main |
| `repo_stash_list`, `repo_stash_save`, `repo_stash_pop`, `repo_stash_drop` | ❌ no `git/stash.rs` on main |
| `repo_list_remotes`, `repo_add_remote` | ❌ `git/remotes.rs` has the fns, but **no commands** |
| `repo_stage_all`, `repo_stage_files` | ❌ `git/staging.rs` has the fns, but **no commands** |
| `repo_fetch`, `repo_pull` | ❌ `git/remotes.rs` has the fns, but **no commands** |
| `repo_diff`, `repo_stage_hunk`, `repo_unstage_hunk` | ⚠️ **defined in `commands/repo.rs` but NOT registered in `lib.rs`** |

**Impact:** the History, Stash, Tags, Remotes, and Staging panels will error at runtime (they invoke non-existent commands). Branches/Changes panels work.

### 5.2 Feature branches with unmerged work
The following branches contain backend modules that were implemented but **never merged into `main`**:

| Branch | Contains |
|--------|----------|
| `feat/stash` | `git/stash.rs` |
| `feat/history` | `git/history.rs`, `git/stash.rs` |
| `feat/tags` | `git/history.rs`, `git/stash.rs`, `git/tags.rs` |
| `feat/tags-advanced` | (tag push/delete, branch publish/upstream) |
| `feat/remotes` | remote management |
| `feat/reflog` | `git/history.rs`, `git/merge.rs`, `git/reflog.rs`, `git/stash.rs`, `git/tags.rs` |
| `feat/merge-rebase` | `git/history.rs`, `git/merge.rs`, `git/stash.rs`, `git/tags.rs` |
| `feat/rebase` | rebase |
| `feat/conflicts` | conflict resolution |
| `feat/cherry-revert` | cherry-pick + revert |
| `feat/diff-integration` | diff viewer wiring |
| `feat/monaco-diff` | Monaco diff viewer |
| `feat/hunk-staging` | hunk staging |
| `feat/fork-sync` | fork sync |
| `feat/github-api` | PR list |
| `feat/actions` | Actions runs |
| `feat/issues` | issues list |
| `feat/pr-workflows` | PR workflows |
| `feat/upstream` | upstream tracking |

> ⚠️ These branches were built on older `main` states and will likely need **rebasing / conflict resolution** before merging. Do not blindly merge.

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

### Immediate / high-value (unblock the UI panels)
1. **Merge/rebuild the unmerged git modules** (stash, tags, history, remotes, staging, merge, reflog) onto current `main`, resolving conflicts.
2. **Register the missing commands** in `lib.rs`:
   - Defined-but-unregistered: `repo_diff`, `repo_stage_hunk`, `repo_unstage_hunk`
   - New commands for: `repo_log`, `repo_list_tags`/`create`/`delete`/`push`, `repo_stash_*`, `repo_list_remotes`/`add`, `repo_stage_all`/`stage_files`, `repo_fetch`/`pull`
3. **Wire the Monaco diff viewer** (from `feat/monaco-diff`).

### Phase 1 remaining (Core Git)
- [ ] Clone/init (HTTPS/SSH)
- [ ] Line-level staging/unstaging
- [ ] Multi-line commit, amend, undo last commit, co-author management
- [ ] Branch rename/delete/publish, upstream tracking
- [ ] History view (virtualized) + diff viewer (unified/side-by-side)
- [ ] Stash (auto-prompt on switch, list/apply/pop/drop)
- [ ] Remote management (add/remove/set-url), fetch/pull

### Phase 2 remaining (Advanced Git + Conflicts)
- [ ] Merge/rebase + conflict resolution UI
- [ ] Interactive rebase, cherry-pick, revert
- [ ] Reflog view
- [ ] Tags (list/create/push/delete)
- [ ] External editor/mergetool/difftool integration
- [ ] Git LFS (basic)

### Phase 3 remaining (GitHub)
- [ ] GHES (Enterprise) API base URL support
- [ ] PR list/view UI (backend commands exist; UI not wired)
- [ ] Actions job log viewing
- [ ] Issues view UI (read-only)

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

1. Reconcile the frontend/backend command mismatch (register missing commands + merge the unmerged git modules) so all panels are functional.
2. Wire the Monaco diff viewer.
3. Implement auto-updater + crash reporting (Phase 4).
4. Investigate AppImage size (87 MB) vs the < 15 MB NFR target.
5. Profile cold start / RAM / UI performance against NFRs.
6. Set a real GitHub OAuth app client ID for production sign-in.
