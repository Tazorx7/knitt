<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="dark-logo.jpg">
    <img src="logo.jpg" alt="" width="140">
  </picture>
</p>

<h1 align="center">knitt</h1>

<p align="center">A native Git client for Linux. Tauri v2, React and libgit2, with no Electron and no system Git.</p>

<p align="center">
  <a href="https://github.com/Tazorx7/knitt/actions/workflows/ci.yml"><img src="https://github.com/Tazorx7/knitt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="Apache-2.0"></a>
</p>

## What it does

Knitt covers the everyday Git loop and the parts people usually drop back to a
terminal for: staging by hunk, amending history, resolving conflicts, and
watching CI, all in one window.

- **Repositories.** Clone over HTTPS or SSH with streamed progress, open or
  initialise a repository, and restore the last one on launch.
- **Changes.** Stage and unstage whole files or single hunks, review diffs in
  Monaco (split or unified, whitespace filter, configurable font/tab size), and
  discard files behind a named confirmation.
- **History.** Virtualised commit log, per-commit changed files, reflog,
  amend, undo the last commit, and co-author trailers.
- **Branches.** Create, rename, delete, publish with upstream tracking, switch
  while stashing local changes, and compare two branches file by file.
- **Remotes and sync.** Fetch, pull (merge, rebase or fast-forward only)
  and push (force-with-lease), per remote, plus tags.
- **Advanced Git.** Merge and rebase with a per-conflict editor and
  Continue/Abort, interactive rebase (pick, reword, squash, fixup, drop),
  cherry-pick, revert, soft/mixed/hard reset, and cleaning untracked files.
- **GitHub.** Device Flow sign-in with multiple accounts (GitHub.com and
  Enterprise), pull requests, issues, and Actions runs with job logs.
- **Integrations.** External editor, difftool and mergetool, detected
  terminal and file manager, and launch at login.
- **Safety and privacy.** Confirmations for every destructive action, a
  system/none/manual network proxy, GPG or SSH commit signing, tokens in the
  Linux Secret Service, and opt-in local-only crash reports with paths
  redacted. No telemetry.

## Requirements

- Linux with WebKitGTK 4.1: Ubuntu 24.04+, Fedora 42+, or Arch (Wayland or X11).
- Rust 1.90+ and Node.js 20+ to build from source.
- `git-lfs` on `PATH` for LFS repositories (optional).

System packages on Debian/Ubuntu:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## Install

Every CI run on `main` builds the bundles and attaches them to the **v0.1.0
draft release**. While that release stays a draft it is only visible to
signed-in users with access to the repository, which is what `gh release
download` handles.

**AppImage (any distribution)**

```bash
gh release download v0.1.0 --pattern '*amd64.AppImage'
chmod +x knitt_0.1.0_amd64.AppImage
./knitt_0.1.0_amd64.AppImage
```

The AppImage needs FUSE. On Ubuntu 24.04 install it with
`sudo apt install libfuse2t64`, or skip FUSE entirely:

```bash
./knitt_0.1.0_amd64.AppImage --appimage-extract-and-run
```

**Debian / Ubuntu**

```bash
gh release download v0.1.0 --pattern '*amd64.deb'
sudo apt install ./knitt_0.1.0_amd64.deb
```

**Fedora**

```bash
gh release download v0.1.0 --pattern '*.x86_64.rpm'
sudo dnf install ./knitt-0.1.0-1.x86_64.rpm
```

The packages add a desktop entry and icon, so Knitt appears in the
application menu. The Uninstall section in Settings removes the package, the
app's data and the saved GitHub tokens; `sudo apt remove knitt` or
`sudo dnf remove knitt` removes only the package and leaves the data behind.
The AppImage is a single file: delete it to remove the app.

### First launch

Knitt opens on an empty window with a repository menu (the round button,
bottom right) offering **Open**, **New** and **Clone**. Everything local
(history, staging, branches, merge and rebase, LFS) works without an account.

The GitHub panels need sign-in, which needs an OAuth client ID compiled into
the build. The bundles above carry one, so sign-in works after install. A
source build reads the ID from `.env` (see below); without it, the app reports
that sign-in is not configured.

## Build from source

```bash
cd ui && npm ci
cd ../src-tauri && cargo tauri dev     # development
cd src-tauri && cargo tauri build      # AppImage, .deb and .rpm bundles
```

The bundles land in `target/release/bundle/`. This path needs the Tauri CLI:
`cargo install tauri-cli --version "^2.0" --locked`.

## GitHub sign-in

Sign-in uses the OAuth **Device Flow** (RFC 8628), which needs a client ID and
nothing else: no client secret, no callback URL. The ID is compiled in at
build time from `.env` or the `GITHUB_CLIENT_ID` environment variable, so
people running Knitt never configure anything: they click *Sign in* and enter
the code at <https://github.com/login/device>.

Maintainers:

1. Create an OAuth App under GitHub → Developer settings.
2. Set any homepage/callback URL (the Device Flow does not use a callback).
3. Enable **Device flow** for that app.
4. Copy the **Client ID** into `.env`:
   ```
   GITHUB_CLIENT_ID=Ov23li...
   ```
   CI reads the same value from the `KNITT_GITHUB_CLIENT_ID` repository
   secret. (GitHub rejects secret names that start with `GITHUB_`, so the
   secret is named for the app and exported as `GITHUB_CLIENT_ID`.)

Without it the app still builds; sign-in reports that GitHub sign-in is not
configured in that build.

## Project layout

| Path | Contents |
|------|----------|
| `ui/` | React 19 + TypeScript frontend, Monaco diff viewer, settings |
| `src-tauri/src/git/` | libgit2 operations: repository, status, staging, history, merge, rebase, tags, stash, reflog, LFS |
| `src-tauri/src/commands/` | Tauri command handlers (repository, settings, auth, GitHub) |
| `src-tauri/src/auth/` | Device Flow, account store, Secret Service access |
| `src-tauri/src/config/` | Versioned settings, diagnostics, crash reports |
| `.github/workflows/` | CI (build, clippy, tests, bundles) and the blue-green deploy |

Repository commands run on a blocking thread, so a slow status or push never
freezes the window.

## Contributing

Work happens on `feat/*` and `fix/*` branches with CI green before merge; see
[CONTRIBUTING.md](CONTRIBUTING.md) and [CONTRIBUTORS.md](CONTRIBUTORS.md). To
report a vulnerability, follow [SECURITY.md](SECURITY.md) instead of opening an
issue.

## License

Apache License 2.0; see [LICENSE](LICENSE).
