# Knitt Settings --- UX & Engineering Specification

**Document:** `setting.md`\
**Product:** Knitt --- lightweight, GitHub-focused desktop Git client
for Linux\
**Status:** Product specification / implementation reference\
**Reference product:** GitHub Desktop\
**Audience:** Product, UI/UX, frontend and Rust/backend implementation

------------------------------------------------------------------------

## 1. Purpose and scope

This document defines the settings experience for Knitt. It uses GitHub
Desktop's documented settings categories as the reference point, then
proposes Linux-oriented extensions where they improve the product.

The distinction matters:

-   **GitHub Desktop reference:** functionality documented by GitHub
    Desktop.
-   **Knitt recommendation:** a proposed feature or implementation
    detail for Knitt. It should not be represented as an existing GitHub
    Desktop feature.

The settings system should make common configuration discoverable
without exposing users to unnecessary Git internals. Advanced controls
should remain available but should explain their consequences.

### Product goals

1.  Let users configure Git identity and behaviour.
2.  Connect and manage GitHub identities securely.
3.  Integrate with Linux editors, terminals and desktop applications.
4.  Make the interface configurable and accessible.
5.  Prevent accidental destructive operations.
6.  Expose enough diagnostics to troubleshoot Git, SSH and network
    problems.
7.  Keep application preferences separate from repository-specific Git
    configuration.

### Non-goals for the first release

-   Replacing a user's system Git configuration manager.
-   Acting as a general-purpose SSH key manager.
-   Silently changing global Git configuration.
-   Storing plaintext passwords, access tokens or private keys in
    Knitt's settings file.
-   Exposing every Git configuration key through a large collection of
    unvalidated toggles.

## 1.1 Backend architecture constraints

Knitt does not shell out to `git`. It links **libgit2** through the
`git2` crate and performs every Git operation in-process. That constrains
which settings can be implemented honestly.

| Fact | Consequence |
| --- | --- |
| Git engine is embedded libgit2 | There is no system Git executable to detect or test. Section 10.1 is not applicable. |
| libgit2 does not execute hooks | Commit, checkout and push never run `.git/hooks/*`. Sections 7.5 and the hook-trust parts of 9.3/15.3 are not applicable until Knitt invokes the system `git` binary. |
| Authentication is the Device Flow (RFC 8628) | No callback URL, no `state` parameter, no PKCE and no client secret. Section 5.1 is written for this flow. |
| The OAuth client ID is compiled in at build time | End users never configure authentication. There must be no client-ID settings field. |
| Secrets live in the Linux Secret Service via `keyring-rs` | Tokens never enter `settings.json`, React state or logs. |
| Settings live in the platform config directory | Persistence must be atomic, schema-versioned and permission-restricted. |

## 1.2 Implementation status and gaps

Status of this specification against the current codebase. Update this
table as work lands; it is the single source of truth for what is real.

| Spec area | Status | Gap |
| --- | --- | --- |
| 5 Accounts: sign-in, sign-out, Secret Service, multiple accounts, Enterprise | Implemented | Accounts section lists accounts, switches the active one, signs out and adds GitHub.com or Enterprise hosts. |
| 2 Settings surface, navigation, `Ctrl+,`, search (2.4) | Implemented | In-app route with a section sidebar, Escape/`Ctrl+,`, and a search field that matches names, descriptions and synonyms and jumps to the control. |
| 4 General | Implemented | Restore-last-repository and default repository directory. No autostart or update controls. |
| 7.1 / 7.2 Global Git name and email | Implemented | Reads the effective global identity; writes only changed keys on explicit save. |
| 7.3 Repository-specific identity | Implemented | Per-repository override writes only the repository's local config, shows the effective source, and resets to global. |
| 7.4 Default branch | Partial | Stored and validated; nothing creates repositories yet, so it has no effect. |
| 8.1 Theme | Implemented | Light/dark/system persisted in the backend; header toggle plus Appearance section. |
| 8.5 Diff tab width | Implemented | Applied to the Monaco diff models; diff theme also follows the app theme now. |
| 6 Integrations: editor, terminal, file manager | Implemented | Detects installed apps, allows a custom executable and argument template, and offers test launches. No shell execution. |
| 9 Safety confirmations | Blocked | Only local branch deletion exists. Discard, reset, clean and force-push are unimplemented, so those toggles would have no effect. |
| 14.4 Schema versioning, migration, atomic writes | Implemented | Schema v2, `#[serde(default)]`, temp-file + fsync + rename, `0600`. |
| 10 Advanced, 11 Accessibility, 12 About | Not implemented | Sections do not exist. |

Correctness issues found while mapping the backend:

- ~~`git/commit.rs` falls back to a dummy signature (`User <user@example.com>`)~~ — fixed; it now reports that identity is unset.
- ~~`repo_push` still pushes a hardcoded `master` ref regardless of the checked-out branch~~ — fixed; it now pushes the checked-out branch with sanitized errors.

------------------------------------------------------------------------

## 2. Settings information architecture

### 2.1 Main navigation

Recommended Knitt sections:

  -----------------------------------------------------------------------
  Section                 Purpose                 Priority
  ----------------------- ----------------------- -----------------------
  General                 Startup, repository     P0
                          defaults and            
                          application behaviour   

  Accounts                GitHub.com and GitHub   P0
                          Enterprise              
                          authentication          

  Integrations            External editor,        P0
                          terminal and Linux      
                          desktop integrations    

  Git                     Commit identity,        P0
                          default branch and Git  
                          hooks                   

  Appearance              Theme, density,         P0
                          typography and diff     
                          preferences             

  Safety                  Confirmation prompts    P0
                          and                     
                          destructive-operation   
                          safeguards              

  Advanced                Git executable,         P1
                          network, diagnostics    
                          and cache               

  Accessibility           Keyboard, screen        P1
                          reader, contrast and    
                          motion preferences      

  About                   Version, build,         P1
                          licences and update     
                          information             
  -----------------------------------------------------------------------

GitHub Desktop's documented core categories are Accounts, Integrations,
Git, Appearance, Prompts, Advanced and Accessibility. General, Safety
and About are proposed Knitt navigation refinements. Knitt may combine
or separate categories differently as the implementation evolves.

### 2.2 Entry point

-   Provide a Settings item in the application menu.
-   Support `Ctrl + ,` on Linux.
-   Make the settings window reachable from the main application without
    losing the current repository context.
-   Use a dedicated settings route or window, rather than mixing
    settings controls into the repository changes view.

### 2.3 Overall layout

**Desktop layout**

-   Left navigation sidebar: fixed or bounded width, approximately
    200--240 px.
-   Main content pane: flexible width, with a readable maximum content
    width.
-   Section title at the top of the content pane.
-   Short explanatory description below the title.
-   Settings grouped into labelled sections.
-   Each setting row contains a label, optional description and control.
-   Use consistent spacing and alignment.
-   Keep destructive or security-sensitive actions visually separate
    from ordinary preferences.

**Suggested hierarchy**

``` text
Settings
├── Search settings
├── General
├── Accounts
├── Integrations
├── Git
├── Appearance
├── Safety
├── Advanced
├── Accessibility
└── About
```

### 2.4 Settings search

**Knitt recommendation**

Provide a search field at the top of the settings navigation or content
area.

Expected behaviour:

-   Search by setting name, description and common synonym.
-   Show matching settings with their section names.
-   Selecting a result opens the relevant section and focuses or scrolls
    to the control.
-   Support keyboard navigation and Escape to clear or close search.
-   Show a useful empty state when nothing matches.
-   Do not search or index secret values, tokens, passwords or private
    key contents.

Examples: - `email` → Git author email. - `editor` → External editor. -
`dark` → Theme. - `ssh` → SSH executable and diagnostics. - `branch` →
Default branch. - `confirm` → Safety prompts.

------------------------------------------------------------------------

## 3. Common interaction and visual specifications

### 3.1 Setting row

Use a consistent row pattern:

-   **Title:** concise, human-readable setting name.
-   **Description:** one sentence explaining the effect.
-   **Control:** right-aligned where space permits.
-   **Status:** optional validation, warning or restart-required text.
-   **Help:** optional tooltip or inline link for technical settings.

Avoid unexplained labels such as `core.autocrlf`. If a raw Git key is
shown, explain what it changes.

### 3.2 Control selection

  Control             Appropriate use
  ------------------- --------------------------------------------------
  Toggle/switch       Independent on/off preference
  Select/dropdown     One choice from a known set
  Radio group         A small number of mutually exclusive choices
  Text input          Name, email, path or other short value
  Path picker         Executable or directory selection
  Segmented control   Small visual choice, such as theme
  Button              Sign in, test, browse, reset or diagnose
  Read-only field     Version, detected path or account identifier
  Warning panel       Risk, validation failure or security consequence

Do not use a toggle for a choice with more than two meaningful states.

### 3.3 Saving behaviour

Recommended behaviour:

-   Apply safe appearance preferences immediately.
-   Persist ordinary preferences automatically.
-   For multi-field or security-sensitive forms, validate first and
    provide an explicit Save/Apply action.
-   Show a concise success or error state.
-   Do not display success until the backend confirms persistence.
-   If a value is invalid, preserve the user's input and explain the
    problem.
-   Provide a per-section reset where practical.
-   A reset must explain what will be restored and whether it affects
    global Git configuration.

### 3.4 Keyboard and focus behaviour

-   All controls must be keyboard reachable.
-   Focus indicators must be visible in both light and dark themes.
-   Tab order should follow visual reading order.
-   Dropdowns and dialogs must support Escape where safe.
-   Do not trap keyboard focus outside modal dialogs.
-   Use accessible names and descriptions for controls.

------------------------------------------------------------------------

# 4. General

**Status:** Knitt recommendation; not one of GitHub Desktop's named core
settings categories.

General contains application-wide defaults that do not belong to Git
identity, authentication or appearance.

## 4.1 Startup behaviour

**UI** - Toggle: `Launch Knitt when I log in` (optional; only if desktop
autostart integration is implemented). - Toggle:
`Restore previously opened repositories`. - Toggle:
`Check for updates automatically` (only if an update mechanism
exists). - Select: startup view, such as Last opened repository or
Repository list.

**Behaviour** - Restore repository selection only if the repository
still exists and is accessible. - If restoration fails, show a
non-blocking message and open the repository list. - Do not launch
background network requests merely because the application starts unless
the user has enabled the relevant feature.

**Security** - Do not automatically open untrusted repositories without
considering repository-controlled content and hooks. - Do not execute
repository hooks on application startup. - Treat repository paths and
metadata as untrusted input.

## 4.2 Default repository directory

**UI** - Read-only path field. - `Browse…` button opening the native
directory picker. - `Open folder` action where supported. - Validation
status for missing or unwritable paths.

**Behaviour** - Use this directory as the initial location in the
clone/create dialogs. - Allow the user to choose another path for an
individual operation. - Do not move existing repositories when this
preference changes.

**Security** - Canonicalise paths before filesystem operations. - Handle
symlinks deliberately; do not assume a displayed path and resolved path
are identical. - Never delete or overwrite an existing directory without
explicit confirmation. - Check permissions before attempting a clone or
repository creation.

## 4.3 Repository restoration and discovery

**UI** - Toggle: restore recently opened repositories. - Optional list
of known local repositories with `Remove from list`. - Optional
`Forget recent history` action.

**Behaviour** - Removing a repository from the list must not delete its
files. - Clearly distinguish `Remove from Knitt` from
`Delete repository from disk`.

**Security** - Recent repository paths can reveal project names and work
activity. Keep this data local and make clearing it straightforward. -
Do not include full local paths in telemetry or diagnostic reports by
default.

------------------------------------------------------------------------

# 5. Accounts

**Reference:** GitHub Desktop Accounts settings.

Purpose: authenticate to GitHub.com and GitHub Enterprise, display
connected identities and allow users to disconnect them.

## 5.1 GitHub.com account

**UI** - Section heading: `GitHub.com`. - Signed-out state:
`Sign in to GitHub.com` button. - Signed-in state: avatar, username,
account host and authentication status. - Action: `Sign out`. - Optional
`Re-authenticate` action if credentials expire or are revoked.

**Behaviour** - Use GitHub's OAuth **Device Flow** (RFC 8628), the
documented flow for native applications that cannot receive a browser
redirect. - Request a device code, show the user code and open
`https://github.com/login/device`; the app polls until the user
authorises or the code expires. - Display a useful error if
authorization is cancelled, denied or fails. - Refresh or renew
credentials only through the supported authentication mechanism.

**Security requirements** - Never ask users to enter their GitHub
account password into Knitt. - Use the minimum OAuth scopes needed for
implemented features, and document why each scope is requested. - Prefer
short-lived access credentials and supported refresh mechanisms. - Store
credentials in the Linux Secret Service/keyring where available. - Do not
store tokens in plaintext JSON, TOML, logs, crash reports or repository
files. - Redact authorization headers, device codes and tokens from
logs. - Use HTTPS for remote GitHub communication. - On sign-out, revoke
credentials where supported and delete the locally stored credential.

**The client ID is not user configuration** - The OAuth client ID is
public and is compiled into the binary at build time from `.env` or the
`GITHUB_CLIENT_ID` build environment variable. There is no settings field
for it. Device flow uses no client secret and no redirect URI, so never
ship a secret and never implement a callback handler.

**Not applicable to device flow** - Callback `state` validation and PKCE
belong to the web application flow. Device flow mitigates the equivalent
risks with a short-lived device code and a user-entered code.

## 5.2 GitHub Enterprise

**UI** - Section heading: `GitHub Enterprise`. - Button:
`Sign in to Enterprise`. - Enterprise host input,
e.g. `https://github.example.com`. - Signed-in account card showing
username and host. - `Sign out` action.

**Behaviour** - Accept only valid HTTPS enterprise URLs by default. -
Validate and normalise the host before beginning authentication. - Keep
enterprise account records distinct from GitHub.com accounts.

**Security** - Do not send enterprise credentials to GitHub.com or
another host. - Restrict authentication and API requests to the
configured enterprise origin. - Do not disable TLS certificate
verification as a workaround. - If self-signed certificates must be
supported in a future release, implement explicit trust configuration
rather than a global TLS bypass.

## 5.3 Multiple accounts

**Knitt recommendation**

Support more than one account, such as personal and work identities.

**UI** - Account list with avatar, username and host. - `Add account`
button. - Per-account `Sign out` or `Remove` action. - Clearly identify
the account used for GitHub API actions.

**Behaviour** - Let users choose an account when an operation requires
one. - Avoid silently switching accounts. - Keep Git commit author
identity separate from GitHub authentication identity.

**Security** - Bind each credential to its host and account
identifier. - Prevent one account's token from being sent to another
host. - Avoid exposing tokens through frontend state or developer
tools. - Use a secure credential store and clear credentials on account
removal.

------------------------------------------------------------------------

# 6. Integrations

**Reference:** GitHub Desktop Integrations settings.

Purpose: configure external applications that Knitt can launch.

## 6.1 External editor

**UI** - Label: `External editor`. - Dropdown of detected editors. -
Option: `Custom editor`. - `Browse…` button to select executable. - Text
input for command-line arguments. - `Test editor` button. - Help text
showing supported path placeholders.

**Potential Linux choices** - Visual Studio Code. - VSCodium. - Zed. -
Sublime Text. - Neovim (if terminal launching is supported). - JetBrains
IDE launchers. - Other detected desktop applications.

Do not hard-code this list as a promise that every editor is installed.
Detect available launchers and allow custom configuration.

**Behaviour** - Open the selected repository or file in the chosen
editor. - Use a documented placeholder such as `%TARGET_PATH%` for the
target path. - Validate that the executable exists and can be
launched. - Show launch errors without crashing Knitt.

**Security** - Never concatenate an editor command into a shell string
and execute it through a shell. - Prefer direct process execution with
an argument vector. - Treat custom executable paths and arguments as
user-controlled input. - Do not execute a repository-provided script as
the editor. - Clearly explain that selecting a custom executable grants
Knitt permission to launch that program. - Do not interpolate untrusted
file paths into shell syntax.

## 6.2 Terminal

**UI** - Dropdown: detected terminal applications. - Custom terminal
executable field. - Argument template field. - `Test terminal` button. -
Help text explaining how the repository path is passed.

**Potential Linux choices** - GNOME Terminal. - Konsole. - Kitty. -
Alacritty. - Xfce Terminal. - Tilix. - Other compatible terminal
emulators.

**Behaviour** - Open a terminal at the selected repository directory. -
Support terminals with different argument conventions through explicit
launch adapters or validated templates. - Show a useful error if the
selected terminal is unavailable.

**Security** - Do not use shell interpolation for repository paths. -
Pass paths as individual process arguments. - Do not run arbitrary
commands merely by opening a terminal. - If a terminal command template
is supported, document its placeholders and validate it carefully.

## 6.3 File manager integration

**Knitt recommendation** - Action to open the current repository in the
system file manager. - Use the desktop environment's supported
file-opening mechanism. - Do not assume GNOME Files is installed.

**Security** - Open the selected directory only. - Do not construct a
shell command from the path. - Handle missing file managers gracefully.

## 6.4 Custom integration validation

For all custom integrations: - Show the resolved executable path. -
Verify existence and executable permissions. - Provide a test action. -
Do not run tests automatically when a repository is opened. - Display
process-launch errors in understandable language. - Do not log
environment variables that may contain credentials.

------------------------------------------------------------------------

# 7. Git

**Reference:** GitHub Desktop Git settings.

This section controls Git configuration used by Knitt. It must
distinguish global configuration from repository-local configuration.

## 7.1 Global author name

**UI** - Text input: `Name`. - Description:
`Used as the author name for commits unless a repository overrides it.` -
Optional Save/Apply action.

**Behaviour** - Read the existing global `user.name`. - Allow the user
to update it. - Explain that this is commit metadata, not the GitHub
account username.

**Security/privacy** - Commit author names are public metadata when
commits are published. - Warn users that the value may appear in public
commit history. - Do not silently replace an existing global value.

## 7.2 Global author email

**UI** - Email input or dropdown of verified/known GitHub-associated
email options when available. - Option to enter another email. - Inline
validation. - Description explaining that the email may appear in commit
metadata.

**Behaviour** - Read and write global `user.email`. - Detect whether the
configured email matches an email associated with the currently
authenticated GitHub account when the API permits. - Show a warning if
it does not match; do not block local commits solely because of the
mismatch.

**Security/privacy** - Explain that commit email addresses can be
visible in public repositories. - Offer the user's GitHub-provided
no-reply address when available. - Do not expose the email in diagnostic
reports unless explicitly included by the user.

## 7.3 Repository-specific identity

**Knitt recommendation based on Git's local configuration model**

**UI** - Current repository identity card. - Choice:
`Use global Git identity`. - Choice:
`Use a custom identity for this repository`. - Name and email inputs
when custom identity is selected. - Status label:
`Inherited from global configuration` or `Repository-specific`.

**Behaviour** - Repository-local configuration overrides global
configuration. - Show the effective identity before commit creation. -
Allow reset to global identity. - Do not change global configuration
when editing a repository-specific identity.

**Security/privacy** - Warn that commit identity is recorded in commit
history. - Validate repository path and ensure the configuration write
targets the intended repository. - Never modify a different repository
due to a stale or mismatched path.

## 7.4 Default branch

**UI** - Text input: `Default branch name`. - Default value: `main`. -
Description: `Used when creating new repositories.` - Validation message
for empty or invalid branch names.

**Behaviour** - This setting affects new repositories created through
Knitt. - It must not rename existing repository branches. - Validate the
branch name using Git's branch-name rules before saving.

**Security** - Do not interpret the branch name as a shell command or
filesystem path. - Reject invalid values and display a useful
explanation.

## 7.5 Git hooks environment

> **Not applicable to the current backend.** Knitt performs Git
> operations through embedded libgit2, which does not execute
> `.git/hooks/*`. No hook environment is loaded and no hook can run, so
> these controls would have no effect. Revisit only if Knitt begins
> invoking the system `git` binary.

**Reference:** GitHub Desktop exposes hook environment options.

Git hooks are executable programs that Git may run during operations
such as commit, checkout or push.

**UI** - Toggle: `Load Git hook environment from login shell`. - Toggle:
`Cache hook environment variables` if environment loading is
implemented. - Explanatory warning that hooks execute local programs. -
Optional `Learn about Git hooks` link.

**Behaviour** - When enabled, obtain the intended shell environment
using a controlled, documented mechanism. - Caching should be
invalidated when relevant configuration changes. - Show hook failures in
the operation output.

**Security** - Git hooks can execute arbitrary local code with the
user's permissions. - Never silently execute hooks from an untrusted
repository without a clear trust policy. - Consider a first-open trust
prompt for repositories with hooks. - Do not assume a hook is safe
because it is stored in `.git/hooks`. - Avoid loading arbitrary shell
startup scripts without explaining the implications. - Do not run hooks
with elevated privileges. - Provide a way to inspect which hook is being
executed. - Never suppress hook errors in a way that makes users believe
an operation succeeded.

## 7.6 Additional Git behaviour

**Knitt extensions --- implement only when supported and tested**

  -----------------------------------------------------------------------
  Setting                 UI                      Behaviour / risk
  ----------------------- ----------------------- -----------------------
  Pull strategy           Select: merge, rebase,  Changes how remote
                          fast-forward only       commits are integrated

  Push default            Select a supported push Can affect which refs
                          mode                    are pushed

  Fetch pruning           Toggle                  Removes stale
                                                  remote-tracking refs;
                                                  explain scope

  Commit signing          Select: off, SSH, GPG,  Requires key and
                          configured              signing setup

  GPG/SSH signing key     Text/path selector      Must not expose private
                                                  key contents

  Git executable          Not applicable to the    Knitt embeds libgit2
                          current backend         and launches no git
                                                  binary

  Auto-fetch              Toggle and interval, if Creates network
                          implemented             activity and uses
                                                  credentials
  -----------------------------------------------------------------------

Do not expose these controls until Knitt can reliably implement,
validate and explain them.

------------------------------------------------------------------------

# 8. Appearance

**Reference:** GitHub Desktop Appearance settings include theme
selection and diff display preferences.

Purpose: let users customise the interface without changing Git
behaviour.

## 8.1 Theme

**UI** - Segmented control or radio group: - Light - Dark - System -
Preview or clear selected state.

**Behaviour** - Apply theme immediately. - System mode follows the
supported Linux desktop preference when available. - If system theme
detection is unavailable, fall back to a documented default. - Persist
the selected mode.

**Quality requirements** - All surfaces must update: window background,
sidebar, dialogs, menus, popovers, dropdowns, tooltips, empty states and
icons. - Avoid hard-coded light-only assets. - Maintain contrast in both
themes. - Avoid flashing the wrong theme during startup.

## 8.2 Accent colour

**Knitt recommendation** - Optional preset accent colours. - Default
system/product accent. - Avoid arbitrary colour selection in the first
release if it complicates contrast guarantees.

**Behaviour** - Apply to focus rings, selected navigation, primary
actions and other semantic accents. - Do not use accent colour as the
only indication of success, error or selection.

## 8.3 Density

**Knitt recommendation** - Comfortable. - Compact.

**Behaviour** - Adjust row heights, list spacing and padding
consistently. - Do not make controls too small to operate. - Preserve
keyboard and touchpad usability.

## 8.4 Typography

**Knitt recommendation** - UI text scale. - Monospace font selection for
diffs and code. - Monospace font size. - Optional line-height
preference.

**Security/quality** - Font selection must not load remote fonts. -
Validate configured font names and use a safe fallback.

## 8.5 Diff display

**Reference:** GitHub Desktop provides a diff tab-size preference.

**UI** - Select/input: `Tab width`. - Optional toggle:
`Show whitespace changes`. - Optional choice: unified or split diff, if
both modes are implemented. - Optional toggle: wrap long lines.

**Behaviour** - Apply to the diff viewer, not to file contents. - Keep
diff display preferences separate from Git's line-ending
configuration. - Make additions, deletions and context lines
distinguishable.

**Accessibility** - Do not communicate added/deleted lines through
colour alone. - Provide symbols, labels or other non-colour
indicators. - Preserve readable contrast in both themes.

------------------------------------------------------------------------

# 9. Safety

**Status:** Knitt recommendation. GitHub Desktop provides Prompts
settings for confirmation dialogs before destructive actions; Knitt can
make the risk model more explicit.

## 9.1 Confirmation preferences

**UI** - Individual confirmation preferences. - Each row contains action
name, short risk explanation and toggle. - A
`Restore recommended defaults` action.

**Suggested actions**

  -----------------------------------------------------------------------
  Action                  Recommended default     Risk
  ----------------------- ----------------------- -----------------------
  Discard one file's      Confirm                 Local uncommitted work
  changes                                         can be lost

  Discard all changes     Confirm                 Multiple local changes
                                                  can be lost

  Delete local branch     Confirm                 Branch reference is
                                                  removed

  Delete remote branch    Confirm                 Shared remote branch is
                                                  removed

  Remove repository from  Confirm                 Removes app
  Knitt                                           registration, not files

  Delete repository from  Strong confirmation     Deletes local data
  disk                                            

  Reset branch            Always confirm          May move or discard
                                                  commits

  Force push              Always confirm          Can overwrite remote
                                                  branch history

  Clean untracked files   Always confirm          Can permanently remove
                                                  files
  -----------------------------------------------------------------------

**Behaviour** - Confirmation dialogs must state the exact target
repository, branch or files. - Destructive dialogs must use specific
verbs, not vague `Continue`. - Separate `Remove from Knitt` from
`Delete from disk`. - For force push, explain the remote branch and
history impact. - Do not allow a global `Never ask again` preference to
disable high-risk confirmations.

## 9.2 Irreversible action safeguards

**Knitt recommendation** - Require a typed confirmation for especially
destructive operations such as deleting a repository from disk. -
Display the exact path before deletion. - Consider a second confirmation
for force push when the target branch is protected or shared. - Where
possible, show recoverability information before reset or branch
deletion.

## 9.3 Repository trust

> **Deferred with 7.5.** Without hook execution there is no repository
> code to gate on open. Repository paths and metadata are still treated
> as untrusted input for path handling and rendering.

**Knitt recommendation** - Display a trust state for repositories. -
Explain that hooks and external tools may execute code. - Ask before
running hooks from a repository that has not been trusted. - Provide a
way to review the repository's configured hooks. - Trust should be
scoped to a repository path and should be revocable.

**Security** - A repository is untrusted input until the user decides
otherwise. - Do not run repository scripts as part of previewing or
opening a repository. - Never elevate permissions to run hooks. - Avoid
broad trust rules such as trusting every repository under a writable
parent directory.

------------------------------------------------------------------------

# 10. Advanced

**Reference:** GitHub Desktop has an Advanced settings category. The
following controls are proposed Knitt capabilities and should be added
only when implemented.

## 10.1 Git executable

> **Not applicable to the current backend.** Knitt embeds libgit2 and
> never launches a `git` executable, so there is nothing to detect,
> validate or test. This section becomes relevant only if a system-Git
> mode is added.

**UI** - Detected Git executable path. - Detected Git version. -
`Browse…` button. - `Detect again` button. - `Test Git` button.

**Behaviour** - Prefer the system Git installation on Linux. - Detect
common installation locations using safe platform APIs and PATH
resolution. - Validate the selected executable by invoking a version
check. - Display the exact executable Knitt will use.

**Security** - Do not execute a binary merely because it exists at a
path. - Resolve and display the selected executable. - Avoid PATH
confusion and unexpected executable substitution. - Do not use a
repository-local executable as the Git binary. - Do not run Git as root.

## 10.2 Network and proxy

**UI** - Proxy mode: system, none, manual (only if supported). - Proxy
URL input. - Optional username field only if required. - Password input
backed by secure credential storage. - `Test connection` button.

**Behaviour** - Clearly indicate whether Knitt uses system proxy
settings or its own configuration. - Test connectivity without exposing
credentials. - Explain which requests are affected.

**Security** - Never store proxy passwords in plaintext settings. -
Redact credentials from URLs and logs. - Require HTTPS for GitHub API
and web traffic. - Do not disable TLS verification. - Prevent proxy
credentials from being sent to unrelated hosts.

## 10.3 SSH diagnostics

**UI** - Detected SSH executable. - SSH agent status. - Public key
filenames or fingerprints, not private key contents. -
`Test SSH connection` action. - Link to SSH troubleshooting
documentation.

**Behaviour** - Diagnose executable availability, agent availability and
authentication failure. - Explain that GitHub SSH authentication is
separate from GitHub browser sign-in. - Do not automatically create or
overwrite keys.

**Security** - Never read or display private key contents in the UI. -
Never upload private keys. - Do not copy private keys into Knitt's
application data directory. - Do not log passphrases or agent
environment secrets. - Do not silently change `~/.ssh/config`.

## 10.4 Diagnostics and logs

**UI** - Current Knitt version. - Git version and executable path. -
Operating system and desktop environment information where relevant. -
`Open logs` action. - `Export diagnostic report` action. - Clear logs
action.

**Behaviour** - Generate a report that users can inspect before
sharing. - Explain where logs are stored. - Allow log clearing. - Avoid
including repository file contents.

**Security** - Redact tokens, passwords, authorization headers, SSH
private data and proxy credentials. - Do not include full environment
variables. - Do not include full repository paths by default. - Do not
include source code, diffs or commit contents unless explicitly selected
by the user. - Keep diagnostic export opt-in. - Make log retention
bounded and documented.

## 10.5 Cache and application data

**UI** - Display cache size where practical. - `Clear cache` action. -
`Reset settings` action. - Explain what each action removes and what it
preserves.

**Behaviour** - Cache clearing must not delete repositories, Git objects
or user credentials. - Resetting settings must not silently delete
account credentials unless the user explicitly chooses that. - Ask for
confirmation before resetting.

**Security** - Keep credentials separate from ordinary cache. - Use
restrictive filesystem permissions for application data. - Avoid
following symlinks outside the intended cache directory during
cleanup. - Use atomic writes for configuration updates.

------------------------------------------------------------------------

# 11. Accessibility

**Reference:** GitHub Desktop provides an Accessibility settings
category. The specific Knitt controls below are recommendations.

## 11.1 Keyboard navigation

**UI** - Toggle or preference: keyboard navigation support (should be on
by default). - Visible shortcut reference. - Configurable shortcuts only
if a reliable shortcut editor is implemented.

**Behaviour** - Every setting must be reachable without a mouse. - Do
not use keyboard shortcuts that conflict with common Linux desktop
shortcuts without a clear reason. - Provide visible focus state.

## 11.2 Screen reader support

**Requirements** - Controls have accessible names. - Descriptions are
associated with the relevant controls. - Status messages are
announced. - Icon-only buttons have labels. - Dialog focus is managed
correctly. - Lists and tables expose meaningful structure.

## 11.3 Contrast and colour

**Requirements** - Maintain sufficient text and control contrast. -
Provide high-contrast support where feasible. - Diff additions and
deletions must remain distinguishable without colour. - Error and
success states must include text or symbols.

## 11.4 Motion

**Knitt recommendation** - Respect system reduced-motion preference. -
Avoid unnecessary animation. - Do not use motion as the only way to
communicate a state change.

------------------------------------------------------------------------

# 12. About

**Knitt recommendation**

**UI** - Knitt logo and version. - Build/channel information. - Git
version. - Links to project website, source repository, issue tracker
and licence information. - Third-party notices and open-source
licences. - Update status if updates are implemented.

**Security** - Do not expose secrets or local paths in About. - Verify
update metadata and packages using a trusted release process. - Do not
execute unverified update binaries.

------------------------------------------------------------------------

# 13. Repository settings are separate

Application settings must not be confused with repository settings.

Application settings describe how Knitt behaves globally. Repository
settings describe the selected local repository.

**Suggested repository settings navigation**

  Section      Contents
  ------------ --------------------------------------------------
  General      Repository name, local path and app registration
  Remotes      Remote names and URLs
  Git Config   Repository-specific identity and configuration
  Branches     Default/tracking branch information
  Git LFS      LFS status and configuration
  Submodules   Submodule status and management
  Hooks        Hook inventory and trust status

### Important distinctions

-   Changing the global author email must not overwrite
    repository-specific identity.
-   Changing the default branch preference must not rename existing
    branches.
-   Removing a repository from Knitt must not delete the repository from
    disk.
-   Editing a remote URL must clearly show which remote is being
    changed.
-   Repository settings must always display the repository name and path
    they apply to.

**Security** - Treat `.git/config`, `.git/hooks`, `.gitmodules`, remote
URLs and repository metadata as untrusted input. - Validate paths and
remote URLs. - Do not execute hook files merely to inspect them. - Warn
before changing a remote host or URL. - Avoid leaking credentials
embedded in remote URLs. - Do not write secrets into `.git/config`.

------------------------------------------------------------------------

# 14. Rust and frontend architecture

Knitt uses a Rust backend and React UI. Settings should have one
authoritative backend service rather than independent persistence logic
in every React component.

## 14.1 Suggested structure

``` text
src/
├── components/
│   └── settings/
│       ├── SettingsWindow.tsx
│       ├── SettingsNavigation.tsx
│       ├── SettingsSearch.tsx
│       ├── SettingsSection.tsx
│       ├── SettingRow.tsx
│       ├── SettingControl.tsx
│       └── sections/
│           ├── GeneralSettings.tsx
│           ├── AccountsSettings.tsx
│           ├── IntegrationsSettings.tsx
│           ├── GitSettings.tsx
│           ├── AppearanceSettings.tsx
│           ├── SafetySettings.tsx
│           ├── AdvancedSettings.tsx
│           ├── AccessibilitySettings.tsx
│           └── AboutSettings.tsx
└── src-tauri/
    └── src/
        ├── settings/
        │   ├── mod.rs
        │   ├── schema.rs
        │   ├── persistence.rs
        │   ├── validation.rs
        │   └── commands.rs
        ├── integrations/
        ├── git/
        └── security/
```

Adapt paths to the actual repository structure and desktop framework.
The important design is separation of concerns, not these exact
filenames.

## 14.2 Settings service responsibilities

Rust should own: - Loading and validating settings. - Applying
defaults. - Migrating old settings schemas. - Persisting settings
atomically. - Reading and writing Git configuration. - Resolving
executable paths. - Launching external applications safely. - Accessing
the operating-system credential store. - Redacting diagnostics. -
Returning structured errors.

React should own: - Rendering controls. - Showing validation messages. -
Managing temporary form state. - Calling backend commands. - Displaying
confirmed persisted state. - Managing focus and keyboard interactions.

## 14.3 Example settings schema

This is an illustrative schema, not a final API contract.

``` rust
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub schema_version: u32,
    pub general: GeneralSettings,
    pub integrations: IntegrationSettings,
    pub git: GitSettings,
    pub appearance: AppearanceSettings,
    pub safety: SafetySettings,
    pub accessibility: AccessibilitySettings,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    pub default_repository_directory: Option<String>,
    pub restore_last_repository: bool,
    pub check_updates: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct IntegrationSettings {
    pub editor_id: Option<String>,
    pub custom_editor_path: Option<String>,
    pub terminal_id: Option<String>,
    pub custom_terminal_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct GitSettings {
    pub default_branch: String,
    pub load_hook_environment: bool,
    pub cache_hook_environment: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,
    pub density: String,
    pub diff_tab_width: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct SafetySettings {
    pub confirm_discard_file: bool,
    pub confirm_discard_all: bool,
    pub confirm_branch_deletion: bool,
    pub confirm_remote_branch_deletion: bool,
    pub confirm_remove_repository: bool,
    pub confirm_delete_repository: bool,
    pub confirm_reset: bool,
    pub confirm_force_push: bool,
    pub confirm_clean_untracked: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AccessibilitySettings {
    pub reduced_motion: bool,
    pub high_contrast: bool,
}
```

Do not store OAuth tokens, passwords, private keys or proxy secrets in
this structure. Store secret references or account identifiers in
ordinary settings, and keep secret material in the operating-system
credential store.

**Current implementation note** - The existing `Settings` struct holds
only `theme`, `git_user_name` and `git_user_email`. It has no
`schema_version`, no `#[serde(default)]` and no atomic write, so adding a
field currently breaks existing settings files. Fix the foundation before
adding any control.

## 14.4 Persistence

Recommended properties: - Versioned schema. - Safe defaults for missing
fields. - Validation on load and before save. - Atomic file
replacement. - Restrictive file permissions. - Corrupt-file recovery
that preserves a backup where appropriate. - Migration tests for every
schema version. - No secrets in the settings file.

Use the platform's standard per-user application configuration
directory. Do not hard-code a home-directory path.

------------------------------------------------------------------------

# 15. Security baseline

Security is not an optional settings category. It is a property of the
entire settings implementation.

## 15.1 Secrets

-   Use Linux Secret Service or another supported OS credential store.
-   Never store OAuth tokens, passwords or private keys in plaintext
    settings.
-   Never include secrets in logs, error strings or diagnostic exports.
-   Do not expose tokens to React unless there is a compelling, reviewed
    requirement.
-   Clear credentials when accounts are disconnected.
-   Avoid copying secrets to the clipboard.

## 15.2 Process execution

-   Use Rust process APIs with argument arrays.
-   Avoid `sh -c`, `bash -c` or equivalent shell execution for ordinary
    integrations.
-   Validate executable paths.
-   Treat repository paths, branch names, remote URLs and custom
    arguments as untrusted.
-   Do not run external tools with elevated privileges.
-   Do not inherit unnecessary environment variables into child
    processes.
-   Apply timeouts and cancellation where appropriate.

## 15.3 Git hooks and repository code

> **Deferred.** libgit2 does not execute hooks, so no repository code can
> run today. These rules apply the moment Knitt invokes the system `git`
> binary or a hook runner.

-   Treat hooks as arbitrary code execution.
-   Do not run hooks simply because a repository is opened.
-   Provide a trust decision and clear explanation.
-   Show which hook is being invoked.
-   Do not suppress failures.
-   Never run hooks as root.
-   Do not assume a repository is trustworthy because it is hosted on
    GitHub.

## 15.4 Network

-   Use HTTPS for GitHub API and web requests.
-   Validate hostnames and authentication destinations.
-   Do not disable certificate validation.
-   Prevent credential forwarding across hosts.
-   Redact tokens and authorization headers.
-   Clearly distinguish GitHub HTTPS authentication from SSH
    authentication.

## 15.5 Filesystem

-   Validate paths before read/write/delete operations.
-   Handle symlinks carefully.
-   Use atomic writes for settings.
-   Apply restrictive permissions to settings and credential-related
    files.
-   Never delete a repository as a side effect of removing it from
    Knitt.
-   Confirm destructive filesystem operations with exact target paths.

## 15.6 Frontend boundary

-   Do not trust values supplied by the React UI.
-   Revalidate all paths, URLs, enum values and operation targets in
    Rust.
-   Expose narrow backend commands rather than a generic arbitrary
    command executor.
-   Avoid rendering untrusted repository content as executable HTML.
-   Do not put credentials in URLs, frontend logs or browser storage.

------------------------------------------------------------------------

# 16. Implementation priorities

## P0 --- first usable settings release

> See 1.2 for current status. Sign-in/sign-out, secure credential storage,
> multiple accounts and GitHub Enterprise already exist in the backend.

-   [ ] Settings window and navigation.
-   [ ] General settings persistence.
-   [ ] GitHub account sign-in/sign-out.
-   [ ] Secure credential storage.
-   [ ] External editor selection.
-   [ ] Terminal selection.
-   [ ] Global Git name and email.
-   [ ] Default branch.
-   [ ] Light, dark and system themes.
-   [ ] Diff tab width.
-   [ ] Destructive-operation confirmations.
-   [ ] Settings schema versioning and migration foundation.
-   [ ] Input validation and structured errors.

## P1 --- polish and Linux integration

-   [ ] Multiple GitHub accounts.
-   [ ] GitHub Enterprise.
-   [ ] Custom editor executable and arguments.
-   [ ] Custom terminal integration.
-   [ ] Repository-specific Git identity.
-   [ ] Git hook environment controls and trust UX.
-   [ ] Default repository directory.
-   [ ] SSH diagnostics.
-   [ ] Git executable detection.
-   [ ] Settings search.
-   [ ] Accessibility improvements.
-   [ ] Diagnostic export with redaction.

## P2 --- advanced workflow

-   [ ] Proxy configuration.
-   [ ] Commit signing preferences.
-   [ ] Pull and push strategy controls.
-   [ ] Fetch pruning.
-   [ ] Git LFS settings.
-   [ ] Submodule controls.
-   [ ] Advanced diff preferences.
-   [ ] Custom accent colours and density.
-   [ ] Update management.
-   [ ] Advanced repository discovery.

Priorities should be revisited against Knitt's actual feature support.
Do not expose a setting before its behaviour is implemented and tested.

------------------------------------------------------------------------

# 17. Acceptance criteria

A settings feature is not complete merely because its control appears on
screen.

### UX acceptance

-   [ ] User can find the setting through navigation or search.
-   [ ] Label and description explain its purpose.
-   [ ] Control is keyboard accessible.
-   [ ] Validation errors are understandable.
-   [ ] Current value is displayed correctly.
-   [ ] Changes provide appropriate feedback.
-   [ ] State remains correct after restarting Knitt.
-   [ ] Light and dark themes are both visually correct.

### Backend acceptance

-   [ ] Rust validates incoming values.
-   [ ] Persistence is atomic.
-   [ ] Missing values receive safe defaults.
-   [ ] Schema migrations are tested.
-   [ ] Errors are structured and actionable.
-   [ ] Git configuration changes target the intended scope.
-   [ ] External processes are launched without unsafe shell
    interpolation.

### Security acceptance

-   [ ] No secrets are stored in plaintext settings.
-   [ ] No secrets appear in logs or diagnostic exports.
-   [ ] Destructive operations require appropriate confirmation.
-   [ ] Repository hooks are treated as executable untrusted code.
-   [ ] Remote credentials are not sent to unrelated hosts.
-   [ ] Paths and executable selections are validated.
-   [ ] The UI cannot bypass backend validation.

------------------------------------------------------------------------

# 18. Official reference material

Use the official documentation to verify GitHub Desktop behaviour as it
evolves.

1.  [Configuring basic settings in GitHub
    Desktop](https://docs.github.com/en/desktop/configuring-and-customizing-github-desktop/configuring-basic-settings-in-github-desktop)
2.  [Configuring Git for GitHub
    Desktop](https://docs.github.com/en/desktop/configuring-and-customizing-github-desktop/configuring-git-for-github-desktop)
3.  [Configuring a default editor in GitHub
    Desktop](https://docs.github.com/en/desktop/configuring-and-customizing-github-desktop/configuring-a-default-editor-in-github-desktop)
4.  [Authenticating to GitHub in GitHub
    Desktop](https://docs.github.com/en/desktop/installing-and-authenticating-to-github-desktop/authenticating-to-github-in-github-desktop)
5.  [GitHub Desktop documentation](https://docs.github.com/en/desktop)

------------------------------------------------------------------------

## Final product principle

Knitt's settings should be powerful without being intimidating.

Common tasks---changing a theme, selecting an editor, configuring commit
identity---should take seconds. Advanced tasks---SSH diagnostics, Git
hooks, proxy configuration and signing---should be discoverable,
explained and safe.

The settings UI must never imply that a feature is supported when the
backend cannot reliably perform it. Every visible control needs a
defined effect, validation rule, persistence path and security model.
