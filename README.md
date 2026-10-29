# knitt

A native Linux Git client built with Tauri v2 + React/TypeScript.

## Features
- GitHub OAuth Device Flow (GitHub.com + GitHub Enterprise)
- Secure token storage via Linux Secret Service (keyring)
- Core Git operations (clone/open, commit, push/pull, branches)
- Staging, stash, history, remotes, tags
- Merge/rebase, cherry-pick/revert, conflict resolution
- GitHub PRs and Issues integration

## Development
```bash
# Install dependencies
cd ui && npm install

# Run dev
cd src-tauri && cargo tauri dev

# Build
cd src-tauri && cargo tauri build
```

## License
Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
