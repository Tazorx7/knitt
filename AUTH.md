# Authentication & GitHub Integration Design

This document outlines how accounts are authenticated, stored, and used by this application. The goal is to match the familiarity of GitHub Desktop's sign in flow whilst keeping everything secure and Linux native. 

> All UI referenced here is to be reimplemented from scratch. No GitHub Desktop assets, branding or trademarks are to be used. 


## 1. Goals

- Support signing in to `github.com` and GitHub Enterprise Server (GHES)
- Use OAuth Device Authorization Flow (RFC 8628) to avoid localhost callback servers
- Keep tokens out of the frontend, localStorage, IndexedDB and config files at all times
- Store tokens securely in the Linux Secret Service via `keyring-rs`
- Allow multi account sign in with the ability to switch between them
- Authenticate both the GitHub API (`octocrab`) and Git operations (push/pull/clone/fetch)
- Gracefully handle offline, revoked tokens, SSO/SAML and environments without a secret service


## 2. Supported Hosts

| Host | API Base | Device Flow Base | Notes |
| :-- | :-- | :-- | :-- |
| `github.com` | `https://api.github.com` | `https://github.com` | Default |
| GitHub Enterprise Server | `https://<host>/api/v3` | `https://<host>` | User provided. Must support self signed certs with explicit consent |


## 3. OAuth App

- We will create our own OAuth App for this project. Do **not** reuse GitHub Desktop's `client_id`.
- Store the `client_id` at build time (e.g `env!`/`tauri` config). Never commit it to the frontend bundle in a way that can be trivially scraped, and never log it.
- Requested scopes (minimal to start): `repo`, `read:org`, `workflow`, `read:user`. Only add scopes when a feature requires them.
- The OAuth App will need to have device flow enabled for both `github.com` and be usable against GHES instances. 


## 4. End to End Login Flow

### Step 1: Host Selection
The sign in modal will present two options: 

1. `Sign in to GitHub.com`
2. `Sign in to GitHub Enterprise Server` - Reveals a text input for the hostname (e.g `git.example.com`). Normalize to scheme + host, strip any paths, validate.

### Step 2: Request Device Code
The Rust backend will hit the appropriate endpoint.

| Host | Endpoint |
| :-- | :-- |
| GitHub.com | `POST https://github.com/login/device/code` |
| GHES | `POST https://<host>/login/device/code` |

Body: 

```json
{
  "client_id": "<OUR_CLIENT_ID>",
  "scope": "repo read:org workflow read:user"
}
```

Response: 

```json
{
  "device_code": "...",
  "user_code": "ABCD-1234",
  "verification_uri": "https://github.com/login/device",
  "expires_in": 900,
  "interval": 5
}
```

### Step 3: Present Code To User
The frontend will display a card with the following: 

- The `user_code` in a large, copyable element
- `verification_uri` with `Copy URL` and `Open in browser` (using `@tauri-apps/plugin-shell`'s `open`)
- Copy explaining to enter the code in the browser
- Cancel button
- Subtle help text

The frontend must never display the `device_code`. Only the backend may hold onto it for polling.

### Step 4: Poll For Access Token
The backend will poll every `interval` seconds on a Tokio task. It must respect `slow_down` (increase interval by +5s) and stop on terminal states.

Endpoint: 

| Host | Endpoint |
| :-- | :-- |
| GitHub.com | `POST https://github.com/login/oauth/access_token` |
| GHES | `POST https://<host>/login/oauth/access_token` |

Body: 

```json
{
  "client_id": "<OUR_CLIENT_ID>",
  "device_code": "<device_code>",
  "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
}
```

Responses: 

| Response | Action |
| :-- | :-- |
| `{"access_token": "...", "token_type": "bearer", "scope": "..."}` | Success. Stop polling, fetch user identity. |
| `{"error": "authorization_pending"}` | Continue polling, respect interval. |
| `{"error": "slow_down"}` | Add +5s to interval, continue polling. |
| `{"error": "expired_token"}` | Show "Code expired, try again." and allow restarting the flow. |
| `{"error": "access_denied"}` | Show "Authorization denied." and return to the host selection screen. | 

### Step 5: Fetch Identity, Persist Securely
On success: 

1. Call the user endpoint with the token: `GET <api_base>/user`. 
2. Construct an `Account` object (no token in this object)
3. Persist the raw access token to the Secret Service via `keyring-rs` keyed by `<host>/<login>` (or a namespaced ID like `github.com:octocat`). 
4. Persist non sensitive metadata to `$XDG_DATA_HOME/<appname>/accounts.json` (id, host, login, name, avatar_url, scopes, added_at, default flags)
5. Emit an `auth:success` event to the frontend with only metadata. The token is never sent over Tauri IPC to JavaScript.


## 5. Secure Storage

### Primary Store (Linux)
- Crate: `keyring` (v3.x)
- Backend: Secret Service over D-Bus (`libsecret`)
- Service name: `org.<appname>.github-auth` (replace `<appname>` with our final app name)
- User/Key: `<host>/<login>` to prevent collisions between hosts and accounts

### Detection
On first sign in attempt, probe the keyring. If the Secret Service is not reachable: 

- Show a blocking, explicit warning explaining secure storage is required for GitHub sign in
- Offer an opt in, encrypted file store ONLY if the user accepts the risk. This must use strong encryption (e.g `age` or `ring` with Argon2) and require a master password that is only held in memory for the session
- If the user does not accept, disable GitHub sign in features and clearly communicate why
- Never write tokens to `config.toml`, `settings.json`, or `localStorage`

### Redaction
- No tokens in `tracing` logs. Always redact `access_token`, `device_code`, and `refresh_token` if ever returned
- Errors from the keyring must be logged without sensitive details
- Crash reports (if opt in) must redact any secrets and prefer repo paths to be minimized


## 6. Git Credential Integration

We need git operations to work with both HTTPS and SSH without asking the user to paste tokens constantly. 

### A) libgit2 Credential Callback (in app git operations)
For all operations executed via `git2` in the Rust backend (fetch, pull, push, status diffing, etc), register `RemoteCallbacks::credentials`.

Logic:
1. Parse the remote URL to extract `host`
2. Resolve the correct GitHub account for this repo/host (see section 7)
3. Fetch the token from the keyring via a Rust only function
4. Return `git2::Cred::userpass_plaintext("x-access-token", &token)`

This is the preferred path for in app actions.

### B) Git Credential Helper (for CLI fallback)
For operations we delegate to the system `git` CLI (e.g. complex interactive rebase, some LFS flows, or edge cases), we need to act as a credential helper.

Plan:
- Ship a small helper command (e.g `<appname>-git-cred`) that implements the Git credential protocol (`get`, `store`, `erase`)
- The helper will look up the token by host from the keyring using the same abstraction
- Configure it repo scoped when needed, or rely on `GIT_ASKPASS`/helper configuration
- Never rely on storing tokens via `git credential-store`
- If the helper cannot find a token for a host, it must exit with a non zero status to allow Git to prompt normally, rather than guessing


### C) SSH
- Do not manage private keys. Rely on `ssh-agent` via `SSH_AUTH_SOCK`
- Respect `~/.ssh/known_hosts` and do not auto accept unknown hosts. Show the OpenSSH style fingerprint to the user before connecting for the first time
- Display loaded keys in `Settings > Git & SSH` as read only information (`ssh-add -l`) for visibility
- Passphrase entry should be handled by `pinentry` through the agent
- For SSH remotes (`git@github.com:...`), the OAuth token is not needed, only the account context for API calls


## 7. Multi Account & Repo Matching

### Account Identity
`GitHubAccount` (metadata only): 

```rust
pub struct GitHubAccount {
  pub id: String,        // e.g "github.com:octocat" or "git.company.com:alice"
  pub host: String,
  pub login: String,
  pub name: Option<String>,
  pub avatar_url: String,
  pub scopes: Vec<String>,
  pub added_at: i64,
  pub is_default_for_host: bool,
}
```

### Resolving Which Account To Use
When an action needs a token for a specific remote/repo: 

1. Parse remote URL -> get `scheme`, `host`, `owner/repo`
2. Find all accounts with matching `host`
3. If zero -> prompt to sign in to that host
4. If exactly one -> use it
5. If multiple -> check for a repo/org specific mapping, else use `is_default_for_host`, else prompt the user to choose (do not pick arbitrarily)
6. Validate access with a lightweight call if ambiguous (e.g `GET /repos/{owner}/{repo}`) and fall back to prompting on 403/404 due to scope

### Account Switcher
- Top bar / repo context: avatar + login with dropdown
- `Settings > Accounts`: list, add account, sign out, set as default per host
- GitHub panel (PRs/Issues/Checks): always shows which account is active for the current host


## 8. Token Lifecycle

| Event | Behavior |
| :-- | :-- |
| App launch | Load accounts metadata. Validate tokens with `GET /user` with short timeout (2-3s). If offline, mark GitHub features as limited. |
| API returns 401 | Mark account as `needs_reauth: true`, show banner "Session invalid for <login> on <host>. Sign in again.", do not delete metadata yet. |
| Token revoked | Same as 401. Restart Device Flow on re-auth. |
| Sign out (single) | Delete token from keyring, remove account from metadata, update active context. |
| Sign out (all) | Delete all tokens from keyring, clear metadata. |
| Rate limited | Respect `X-RateLimit-Remaining` and `X-RateLimit-Reset`. Show subtle UI if approaching limit. Do not retry aggressively. |


## 9. Tauri Command Surface

Keep the IPC surface minimal and secret safe. None of these should return raw tokens to the frontend. 

| Command | Purpose |
| :-- | :-- |
| `auth_start_device_flow(host: Option<String>)` | Start flow for github.com or GHES. Returns `user_code`, `verification_uri`, `expires_in`. |
| `auth_poll_device_flow()` | Called by backend polling or polled via events. Advances the flow state. |
| `auth_cancel_device_flow()` | Cancel polling and reset flow state. |
| `auth_list_accounts()` | Return account metadata array only. |
| `auth_set_active_account(account_id: String)` | Switch active account context. |
| `auth_sign_out(account_id: String)` | Remove token + metadata for account. |
| `auth_sign_out_all()` | Remove all. |
| `auth_get_accounts_for_host(host: String)` | Return metadata for accounts that match host (for repo context). |

State updates will be pushed via `app.emit` events: `auth:pending`, `auth:code`, `auth:success`, `auth:error`, `auth:expired`.


## 10. Security Checklist

- [ ] Tokens never serialized to frontend, never in `localStorage`/`sessionStorage`/IndexedDB
- [ ] Tokens never logged by `tracing`. Redaction in place
- [ ] Only allowlisted Tauri commands exposed
- [ ] HTTPS only for GitHub/GHE API (dev only exception must be explicit and disabled in release)
- [ ] Self signed GHES certs require user confirmation with host and cert details
- [ ] OAuth scopes kept minimal
- [ ] Keyring namespace is unique to this app
- [ ] Sign out fully deletes from keyring
- [ ] User codes/verification URIs only shown, never tokens
- [ ] Polling cancelled on modal close/unmount
- [ ] Secret store fallback is opt in and clearly warned about
- [ ] No global "get secret" command


## 11. Implementation Order

| Order | Task | Notes |
| :-- | :-- | :-- |
| 1 | Secret store abstraction (`src-tauri/src/auth/secret_store.rs`) | keyring + fallback detection first |
| 2 | Device Flow for github.com | End to end against real API, test with own account |
| 3 | Account model + persistence | `accounts.json` + metadata only |
| 4 | libgit2 credential callback | Make push/pull/fetch actually work |
| 5 | Git credential helper | Fallback for CLI routed operations |
| 6 | GHES support + self signed cert handling | Host normalization + API base override + consent modal |
| 7 | Multi account UI + switcher | Device code modal, account switcher, settings page |
| 8 | Token lifecycle validation | Startup validation, 401 handling, re auth | 
