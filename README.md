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

# Configure GitHub sign-in (one-time, maintainers only)
cp .env.example .env
# put your OAuth app's Client ID in .env — see "GitHub sign-in" below

# Run dev
cd src-tauri && cargo tauri dev

# Build
cd src-tauri && cargo tauri build
```

### GitHub sign-in

Sign-in uses the OAuth **Device Flow** (RFC 8628), which needs **only a client ID**
— no client secret and no redirect URL. The client ID is compiled into the app
from `.env` (gitignored) or the `GITHUB_CLIENT_ID` environment variable, so end
users never configure anything; they just click *Sign in* and enter the code at
<https://github.com/login/device>.

To set it up (maintainers):
1. Create an OAuth App at GitHub → Developer settings.
2. Set any homepage/callback URL (device flow does not use a callback).
3. Enable **Device flow** in the app's settings.
4. Copy the **Client ID** into `.env`:
   ```
   GITHUB_CLIENT_ID=Ov23li...
   ```
   CI reads it from the `GITHUB_CLIENT_ID` repository secret.

If it is unset, the app still builds but reports "GitHub sign-in is not
configured in this build."

## License
Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
