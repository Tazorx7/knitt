# Product Requirements Document (PRD): Native Linux Git Client (Tauri Edition)

## 1. Document Overview & Executive Summary

This PRD outlines the functional, technical, and operational requirements for a high-performance, open-source native Linux Git client built with Tauri v2 (Rust backend + WebKitGTK webview) and a custom React/TypeScript frontend.

**Design Philosophy:**
- **Familiar UX:** Adopt the same information architecture, panel layout, and interaction patterns as GitHub Desktop to provide a frictionless transition for its users (akin to the relationship between VS Code and Code - OSS).
- **Clean-room reimplementation:** All UI components, styles, logic, icons, and assets must be implemented from scratch. No GitHub Desktop CSS, SVGs, images, or layout files will be copied.
- **Distinct identity:** The application will ship under its own unique name, bundle ID, app icon, and branding with zero GitHub Desktop trademarks or wordmarks.
- **Electron-free by design:** Strip all Electron-specific patterns and replace with Tauri's Rust command system with strict IPC scoping.
- **Performance-first:** Target < 50 MB idle RAM, < 15 MB packaged binary (with aggressive optimization), and < 400 ms cold start on NVMe SSDs.
- **Linux-native:** Leverage native secret storage (libsecret/Secret Service), respect XDG portals (Wayland), and integrate properly with Linux desktop environments.

**Reference:** See `AUTH.md` for the complete authentication design (OAuth Device Flow, keyring storage, git credential integration).


## 2. Product Objectives & Target Audience

- **Target Audience:** Linux developers, open-source maintainers, and system engineers who want a fast, native Git/GitHub client without Electron overhead.
- **Primary Goal:** Provide a coherent, discoverable Git workflow (clone/init, stage/commit, branch/merge/rebase, stash, history) with first-class GitHub integration while maintaining excellent performance on Linux.
- **Secondary Goal:** Maintain a clean, extensible architecture that can grow without compromising startup time or memory footprint.


## 2.5 Branding, Trademarks & Legal

| Item | Requirement |
| :-- | :-- |
| **App Name** | Use an original, neutral name. "GitHub Desktop" must never appear in the app title, window title, about page, or CLI. |
| **Branding** | Custom logomark and wordmark. No GitHub, GitHub Desktop, or GitHub Inc. logos. |
| **Bundle ID** | Use a reverse-DNS ID unique to this project (e.g `app.<chosenname>.<project>`), not `com.github.desktop`. |
| **UI Reference** | GitHub Desktop is used strictly as a UX reference for workflows and IA. Layouts may feel identical, but implementation must be original. |
| **Assets** | Only original icons, illustrations, and fonts. No assets pulled from the `desktop/desktop` repository. |
| **Affiliation** | The app must not claim to be "GitHub Desktop for Linux" or endorsed by GitHub. "GitHub integration" is acceptable to describe API connectivity. |
| **Licensing** | Follow our own license. Be mindful of `desktop/desktop`'s MIT license, but do not rely on copying files to satisfy it. |


## 3. MVP Scope & Phasing

### Phase 0 (Foundations) - MVP Blockers
- [ ] Tauri v2 scaffolding with Rust backend and React/TypeScript frontend
- [ ] App settings and config system (TOML/JSON) with XDG paths
- [ ] Logging/diagnostics infrastructure (`tracing`)
- [ ] Secure secret storage abstraction (keyring-rs with libsecret) + fallback strategy
- [ ] Strict Tauri IPC allowlist + path validation
- [ ] File watching (inotify) for repository changes

### Phase 1 (Core Git) - MVP
- [ ] Local repo discovery/open, init, clone (HTTPS/SSH)
- [ ] Status, staging (file/hunk/line), unstaging
- [ ] Commit (multi-line, amend, undo last commit), co-author management
- [ ] Branch create/switch/rename/delete/publish, upstream tracking
- [ ] History view (virtualized), diff viewer (unified/side-by-side)
- [ ] Stash (auto-prompt on switch, list/apply/pop/drop)
- [ ] Remote management (add/remove/set-url), fetch/pull/push

### Phase 2 (Advanced Git + Conflicts) - MVP+
- [ ] Merge/rebase (basic), conflict detection with resolution UI
- [ ] Interactive rebase, cherry-pick, revert
- [ ] Reflog view
- [ ] Tags (list/create/push/delete)
- [ ] External editor/mergetool/difftool integration
- [ ] Git LFS (basic: track/untrack, pull/push, lock list) with CLI fallback

### Phase 3 (GitHub Integration) - MVP+
- [ ] OAuth Device Flow for GitHub.com + GitHub Enterprise Server (see `AUTH.md`)
- [ ] Multi-account switching with token lifecycle management
- [ ] PR list/view/create/checkout
- [ ] Fork workflows (sync with upstream)
- [ ] GitHub Actions run status + basic job log viewing
- [ ] Issues list/view (read-only initially)

### Phase 4 (Polish & Distribution) - Release
- [ ] Themes (light/dark/system), keyboard shortcuts, accessibility pass
- [ ] Auto-updater (Tauri updater) with signatures
- [ ] Crash reporting (opt-in, privacy-first)
- [ ] Packaging (.deb/.rpm/.AppImage), Flathub, AUR
- [ ] Performance profiling against NFRs

**Definition of "feature-equivalent":** Equivalent *workflows and information architecture* to GitHub Desktop v3.x as of late 2025/early 2026, not pixel-for-pixel. The UI must *feel the same* to use. Visuals, assets, and branding must be original.


## 4. Technical Architecture & System Design

### 4.1 High-Level Architecture
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    FRONTEND (React 19 + TypeScript)                         │
│  - Custom UI Components (no GitHub Desktop branding)                        │
│  - Monaco Editor (Diff/Staging) - lazy-loaded                               │
│  - Zustand (multi-repo + UI state)   - TanStack Virtual (lists)             │
│  - Tailwind CSS + Radix UI primitives (unstyled/accessible)                 │
│  - React Router / view state machine                                        │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                        │ Tauri v2 IPC (invoke/emit) - allowlisted
┌──────────────────────────────────────▼───────────────────────────────────────┐
│                            BACKEND (Rust Core)                               │
│  ┌─────────────────────────┐  ┌──────────────────────────────────────────┐  │
│  │   git2 (libgit2)        │  │ std::process::Command (CLI Fallback)     │  │
│  │ status/diff/index/commit │  │ rebase/merge advanced, LFS, SSH/agent,    │  │
│  │ branches/tags/blame     │  │ git hooks passthrough, complex ops       │  │
│  └─────────────────────────┘  └──────────────────────────────────────────┘  │
│  ┌─────────────────────────┐  ┌──────────────────────────────────────────┐  │
│  │ keyring-rs + Secret Svc │  │ octocrab (GitHub REST/GraphQL)           │  │
│  │ tokens/creds (encrypted)│  │ OAuth device flow, PRs/Issues/Actions     │  │
│  └─────────────────────────┘  └──────────────────────────────────────────┘  │
│  ┌─────────────────────────┐  ┌──────────────────────────────────────────┐  │
│  │ FS Watcher (notify)     │  │ Config/Logging/Updater/Crash (opt-in)     │  │
│  │ inotify on Linux        │  │ XDG paths, tracing, Tauri updater        │  │
│  └─────────────────────────┘  └──────────────────────────────────────────┘  │
└──────────────────────────────────────┬───────────────────────────────────────┘
                                        │ Native Linux APIs
┌──────────────────────────────────────▼───────────────────────────────────────┐
│                        Linux OS Runtime                                     │
│  WebKitGTK • D-Bus Secret Service • XDG Portals • inotify • ssh-agent/gpg   │
│  X11/Wayland • pinentry • credential helpers                                 │
└──────────────────────────────────────────────────────────────────────────────┘
```


## 5. Non-Functional Requirements (NFRs)

| Metric | Target | Validation Method |
|--------|--------|-------------------|
| **Idle Memory Footprint** | < 50 MB RAM | Measure with `ps_mem`/`htop` after idle |
| **Binary Package Size** | < 15 MB | Check release build size for .deb/.AppImage |
| **Cold Startup Latency** | < 400 ms | Time from launch to UI ready on NVMe SSD |
| **UI Performance** | 60fps scrolling @ 100k+ commits | Profile with dev tools, TanStack Virtual benchmarks |
| **Security** | Zero plaintext tokens, strict IPC validation | Security review, audit logging |
| **Reliability** | Graceful error handling, no crashes on common failures | Error boundary, recovery flows |
| **Compatibility** | Git >= 2.30, libgit2 compatible, WebKitGTK on supported distros | Test matrix (Ubuntu 24.04, Fedora 42, Arch) |


## 6. Distribution & Packaging Strategy

| Channel | Details | Status |
|---------|---------|--------|
| **.deb** | Debian/Ubuntu/Pop!_OS, built via `tauri-action`, proper dependencies | P0 |
| **.rpm** | Fedora/RHEL/openSUSE, RPM spec via tauri-action | P0 |
| **.AppImage** | Universal, bundle dependencies where possible | P0 |
| **Flathub** | Flatpak manifest, use Freedesktop runtime, portal permissions, Secret Service access | P1 |
| **AUR** | PKGBUILD for Arch/Manjaro, community maintained | P1 |
| **Auto-updates** | Tauri updater with signed releases | P1 |


## 7. Success Criteria

- [ ] UI layout and flows are immediately familiar to GitHub Desktop users
- [ ] Zero instances of "GitHub Desktop", GitHub Desktop logos, or copied assets in the codebase or built app
- [ ] Original app name, icon, and bundle ID used across all platforms
- [ ] Cold start < 400ms, idle RAM < 50MB on test machine
- [ ] Packaged binary < 15MB
- [ ] Can clone, commit, push/pull, branch, merge basic conflicts on Linux
- [ ] GitHub auth + PR checkout works on github.com (via OAuth Device Flow as specified in `AUTH.md`)
- [ ] Passes basic security review (no plaintext tokens, IPC validation)
- [ ] Installs and runs on at least Ubuntu 24.04, Fedora 42, Arch (Wayland + X11 tested)
