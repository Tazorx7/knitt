//! Removing Knitt itself: the package or AppImage first, then every file and
//! keyring entry the app created. The plan is read-only and shown before the
//! user confirms, so nothing is guessed at removal time.

use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Package name, as produced by `tauri.conf.json` (`productName`).
const PACKAGE: &str = "knitt";
/// Directory under the XDG base directories, matching `config::paths`.
const APP_DIR: &str = "nativegitclient";
/// Tauri's identifier, which names the webview's storage directory.
const IDENTIFIER: &str = "app.nativegitclient.desktop";
/// Keyring service name, matching `auth::secret_store`.
const KEYRING_APP: &str = "nativegitclient";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallKind {
    Deb,
    Rpm,
    AppImage,
    Source,
}

impl InstallKind {
    fn label(self) -> &'static str {
        match self {
            Self::Deb => "Debian package",
            Self::Rpm => "RPM package",
            Self::AppImage => "AppImage",
            Self::Source => "Source build",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RemovalItem {
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UninstallPlan {
    pub kind: InstallKind,
    pub kind_label: String,
    pub package: Option<String>,
    /// True when removing the install needs the system password prompt.
    pub needs_password: bool,
    /// Human-readable system-level steps, e.g. "Remove the knitt package".
    pub system_items: Vec<String>,
    /// Files and directories that exist and will be deleted.
    pub data_items: Vec<RemovalItem>,
    /// Saved GitHub accounts whose keyring tokens will be deleted.
    pub keyring_accounts: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct UninstallOutcome {
    /// sudo rejected the password; nothing was changed.
    pub auth_failed: bool,
    /// The package or AppImage is gone.
    pub system_removed: bool,
    pub data_removed: Vec<String>,
    /// Steps that failed after the system removal succeeded.
    pub failures: Vec<String>,
}

/// What an uninstall would remove on this machine. Reads state, changes nothing.
pub fn plan() -> UninstallPlan {
    let appimage = appimage_path();
    let kind = classify(
        appimage.as_deref(),
        running_from_appimage_mount(),
        deb_installed(),
        rpm_installed(),
    );
    let (system_items, requires_root) = system_items(kind, appimage.as_deref());

    UninstallPlan {
        kind,
        kind_label: kind.label().to_string(),
        package: matches!(kind, InstallKind::Deb | InstallKind::Rpm).then(|| PACKAGE.to_string()),
        needs_password: requires_root && password_required(),
        system_items,
        data_items: data_paths()
            .into_iter()
            .filter(|(path, _)| path.exists())
            .map(|(path, description)| RemovalItem {
                path: path.display().to_string(),
                description: description.to_string(),
            })
            .collect(),
        keyring_accounts: saved_accounts().len(),
    }
}

/// Removes the install, then the app's own data. Returns `Ok` with
/// `cancelled` set when the user dismissed the password dialog.
pub fn run(password: Option<&str>) -> Result<UninstallOutcome, String> {
    let appimage = appimage_path();
    let kind = classify(
        appimage.as_deref(),
        running_from_appimage_mount(),
        deb_installed(),
        rpm_installed(),
    );
    let mut outcome = UninstallOutcome::default();

    // Package files live in /usr, so that step needs root and runs first: a
    // rejected password must leave everything untouched.
    if let Some((program, args)) = purge_command(kind) {
        let program = find_in_path(program)
            .ok_or_else(|| format!("{program} is not installed on this system."))?;
        match purge(&program, args, password) {
            Ok(()) => outcome.system_removed = true,
            Err(PurgeFailure::Auth) => {
                outcome.auth_failed = true;
                return Ok(outcome);
            }
            Err(PurgeFailure::Message(message)) => return Err(message),
        }
    }

    let (removed, failures) = remove_data();
    outcome.data_removed = removed;
    outcome.failures = failures;

    // An AppImage is a single file, so it goes last: unlinking the image while
    // this process still reads from it is the one step that can pull the
    // ground out from under us.
    if kind == InstallKind::AppImage {
        match &appimage {
            Some(path) => match remove_path(path) {
                Ok(()) => outcome.system_removed = true,
                Err(message) => outcome.failures.push(message),
            },
            None => outcome.failures.push(
                "Could not find the AppImage file to delete; remove it yourself.".to_string(),
            ),
        }
    }

    Ok(outcome)
}

fn system_items(kind: InstallKind, appimage: Option<&Path>) -> (Vec<String>, bool) {
    match kind {
        InstallKind::Deb => (
            vec![format!(
                "Remove the {PACKAGE} package and the files it installed (dpkg --purge)"
            )],
            true,
        ),
        InstallKind::Rpm => (
            vec![format!(
                "Remove the {PACKAGE} package and the files it installed (rpm -e)"
            )],
            true,
        ),
        InstallKind::AppImage => (
            vec![match appimage {
                Some(path) => format!("Delete {}", path.display()),
                None => "Delete the AppImage file (its path was not found)".to_string(),
            }],
            false,
        ),
        InstallKind::Source => (
            vec![
                "Nothing system-wide: a source build is removed by deleting its folder"
                    .to_string(),
            ],
            false,
        ),
    }
}

fn classify(
    appimage: Option<&Path>,
    mounted: bool,
    deb: bool,
    rpm: bool,
) -> InstallKind {
    if appimage.is_some() || mounted {
        return InstallKind::AppImage;
    }
    if deb {
        return InstallKind::Deb;
    }
    if rpm {
        return InstallKind::Rpm;
    }
    InstallKind::Source
}

fn purge_command(kind: InstallKind) -> Option<(&'static str, &'static [&'static str])> {
    match kind {
        InstallKind::Deb => Some(("dpkg", &["--purge", PACKAGE])),
        InstallKind::Rpm => Some(("rpm", &["-e", PACKAGE])),
        _ => None,
    }
}

/// The AppImage the user launched, when the runtime tells us where it is.
fn appimage_path() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
}

fn running_from_appimage_mount() -> bool {
    std::env::current_exe()
        .is_ok_and(|path| path.to_string_lossy().contains("/.mount_"))
}

fn deb_installed() -> bool {
    Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", PACKAGE])
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("install ok installed")
        })
}

fn rpm_installed() -> bool {
    Command::new("rpm")
        .args(["-q", PACKAGE])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn saved_accounts() -> Vec<crate::auth::accounts::GitHubAccount> {
    crate::auth::accounts::AccountsStore::load()
        .map(|store| store.accounts)
        .unwrap_or_default()
}

/// Every path Knitt writes outside its own binary. Only existing ones are
/// listed or removed.
fn data_paths() -> Vec<(PathBuf, &'static str)> {
    let mut paths = Vec::new();
    if let Some(base) = dirs::config_dir() {
        push_config(&mut paths, &base);
    }
    if let Some(base) = dirs::data_dir() {
        push_data(&mut paths, &base);
    }
    if let Some(base) = dirs::state_dir() {
        paths.push((base.join(APP_DIR), "Logs"));
    }
    if let Some(base) = dirs::cache_dir() {
        push_cache(&mut paths, &base);
    }
    paths
}

fn push_config(out: &mut Vec<(PathBuf, &'static str)>, base: &Path) {
    out.push((
        base.join(APP_DIR),
        "Settings, recent repositories and the saved account list",
    ));
    out.push((
        base.join("autostart").join("knitt.desktop"),
        "Autostart entry",
    ));
}

fn push_data(out: &mut Vec<(PathBuf, &'static str)>, base: &Path) {
    out.push((base.join(APP_DIR), "Saved accounts and crash reports"));
    out.push((base.join(IDENTIFIER), "Webview storage and cache"));
}

fn push_cache(out: &mut Vec<(PathBuf, &'static str)>, base: &Path) {
    out.push((base.join(APP_DIR), "Cache"));
    out.push((base.join(IDENTIFIER), "Webview cache"));
}

fn remove_data() -> (Vec<String>, Vec<String>) {
    // Read the account list first. Loading it goes through `paths::data_dir()`,
    // which creates that directory, so reading it afterwards would put back a
    // folder the removal just deleted.
    let accounts = saved_accounts();

    let (removed, mut failures) = remove_paths(&data_paths());

    // Keyring entries live outside the filesystem.
    let secret = crate::auth::secret_store::SecretStore::new(KEYRING_APP);
    for account in accounts {
        if let Err(error) = secret.delete_token(&account.id) {
            tracing::warn!(%error, account = %account.id, "could not remove the saved token");
            failures.push(format!(
                "Could not remove the saved GitHub token for {}.",
                account.login
            ));
        }
    }

    (removed, failures)
}

/// Deletes every listed path that still exists, collecting the ones that do
/// not come back clean instead of stopping at the first failure.
fn remove_paths(paths: &[(PathBuf, &'static str)]) -> (Vec<String>, Vec<String>) {
    let mut removed = Vec::new();
    let mut failures = Vec::new();

    for (path, _) in paths {
        if !path.exists() {
            continue;
        }
        match remove_path(path) {
            Ok(()) => removed.push(path.display().to_string()),
            Err(message) => failures.push(message),
        }
    }

    (removed, failures)
}

/// Deletes a file, symlink or directory. A symlink is unlinked rather than
/// followed, so a stray link can never delete its target.
fn remove_path(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        tracing::warn!(%error, path = %path.display(), "could not inspect app data");
        format!("Could not remove {}.", path.display())
    })?;
    let result = if metadata.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    result.map_err(|error| {
        tracing::warn!(%error, path = %path.display(), "could not remove app data");
        format!("Could not remove {}.", path.display())
    })
}

/// Why the package manager did not run.
#[derive(Debug, PartialEq, Eq)]
enum PurgeFailure {
    /// sudo rejected the password the user typed.
    Auth,
    /// Something the user needs to read: no sudo, no permission, a dpkg error.
    Message(String),
}

const TERMINAL_HINT: &str = "Remove the package from a terminal with `sudo dpkg --purge knitt` or `sudo rpm -e knitt`, then delete the folders listed above.";

/// Runs the package manager as root. `password` is the sudo password the user
/// typed in the dialog; `None` means a cached credential or a NOPASSWD rule
/// should cover it.
fn purge(program: &Path, args: &[&str], password: Option<&str>) -> Result<(), PurgeFailure> {
    let sudo = find_in_path("sudo").ok_or_else(|| {
        PurgeFailure::Message(format!("sudo is not installed. {TERMINAL_HINT}"))
    })?;

    if password.is_none() {
        let output = Command::new(&sudo)
            .arg("-n")
            .arg("--")
            .arg(program)
            .args(args)
            .output()
            .map_err(|_| PurgeFailure::Message("Could not run sudo.".to_string()))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(code = output.status.code(), "package removal failed");
        return Err(classify_sudo_failure(&stderr, output.status.code()));
    }

    let Some(password) = password else {
        return Err(PurgeFailure::Auth);
    };

    let mut child = Command::new(&sudo)
        // -S reads the password from stdin, -k ignores a cached credential so
        // the field is authoritative, and -p "" keeps the prompt off stderr.
        .arg("-S")
        .arg("-k")
        .arg("-p")
        .arg("")
        .arg("--")
        .arg(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| PurgeFailure::Message("Could not run sudo.".to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        let mut line = password.as_bytes().to_vec();
        line.push(b'\n');
        // A closed pipe means sudo finished without needing the password (a
        // NOPASSWD rule, for instance), so the write error is not a failure.
        let _ = stdin.write_all(&line);
        // The password is not stored anywhere, so wipe the copy we made.
        line.fill(0);
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .map_err(|_| PurgeFailure::Message("sudo did not finish.".to_string()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    tracing::warn!(code = output.status.code(), "package removal failed");
    Err(classify_sudo_failure(&stderr, output.status.code()))
}

/// Turns sudo or dpkg output into the one thing the dialog can act on: ask
/// again for the password, or show what went wrong.
fn classify_sudo_failure(stderr: &str, code: Option<i32>) -> PurgeFailure {
    let lower = stderr.to_lowercase();
    if lower.contains("incorrect password")
        || lower.contains("sorry, try again")
        || lower.contains("authentication failed")
        || lower.contains("authentication required but not attempted")
        || lower.contains("interactive authentication is required")
        || lower.contains("a password is required")
        || lower.contains("no password was provided")
    {
        return PurgeFailure::Auth;
    }
    if lower.contains("not in the sudoers file")
        || lower.contains("no tty")
        || lower.contains("a terminal is required")
    {
        return PurgeFailure::Message(TERMINAL_HINT.to_string());
    }
    match stderr.lines().find(|line| !line.trim().is_empty()) {
        Some(line) => PurgeFailure::Message(format!(
            "Could not remove the package: {}",
            line.trim()
        )),
        None => PurgeFailure::Message(match code {
            Some(code) => format!("Could not remove the package (sudo exited with {code})."),
            None => "Could not remove the package.".to_string(),
        }),
    }
}

/// Whether sudo would ask for a password right now. Read-only: `-n` never
/// prompts, so this is safe to call while building the plan.
fn password_required() -> bool {
    let Some(sudo) = find_in_path("sudo") else {
        return true;
    };
    Command::new(sudo)
        .args(["-n", "--", "true"])
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(true)
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_install_wins_over_the_others() {
        let image = Path::new("/home/u/knitt.AppImage");
        assert_eq!(
            classify(Some(image), false, true, true),
            InstallKind::AppImage
        );
        assert_eq!(classify(None, true, true, true), InstallKind::AppImage);
        assert_eq!(classify(None, false, true, true), InstallKind::Deb);
        assert_eq!(classify(None, false, false, true), InstallKind::Rpm);
        assert_eq!(classify(None, false, false, false), InstallKind::Source);
    }

    #[test]
    fn package_managers_get_the_right_command() {
        assert_eq!(
            purge_command(InstallKind::Deb),
            Some(("dpkg", &["--purge", "knitt"] as &[&str]))
        );
        assert_eq!(
            purge_command(InstallKind::Rpm),
            Some(("rpm", &["-e", "knitt"] as &[&str]))
        );
        assert_eq!(purge_command(InstallKind::AppImage), None);
        assert_eq!(purge_command(InstallKind::Source), None);
    }

    #[test]
    fn data_paths_cover_every_location_the_app_writes() {
        let base = Path::new("/base");
        let mut paths = Vec::new();
        push_config(&mut paths, base);
        push_data(&mut paths, base);
        push_cache(&mut paths, base);
        let listed: Vec<String> = paths.iter().map(|(p, _)| p.display().to_string()).collect();
        assert_eq!(
            listed,
            vec![
                "/base/nativegitclient",
                "/base/autostart/knitt.desktop",
                "/base/nativegitclient",
                "/base/app.nativegitclient.desktop",
                "/base/nativegitclient",
                "/base/app.nativegitclient.desktop",
            ]
        );
    }

    #[test]
    fn a_wrong_password_is_reported_as_auth_failure() {
        for stderr in [
            // What sudo 1.9 actually prints with -S -k and a wrong password.
            "\nsudo: Authentication failed, try again.\n\nsudo: Authentication required but not attempted\n",
            // What sudo -n prints when no credential is cached.
            "sudo: interactive authentication is required",
            "Sorry, try again.",
            "sudo: a password is required",
            "sudo: 3 incorrect password attempts",
        ] {
            assert_eq!(classify_sudo_failure(stderr, Some(1)), PurgeFailure::Auth, "{stderr}");
        }
    }

    #[test]
    fn other_sudo_failures_carry_a_message() {
        match classify_sudo_failure(
            "dpkg: error: dpkg frontend lock was locked by another process",
            Some(2),
        ) {
            PurgeFailure::Message(message) => {
                assert!(message.contains("dpkg frontend lock"), "{message}")
            }
            PurgeFailure::Auth => panic!("a lock is not an auth failure"),
        }
        match classify_sudo_failure("", Some(4)) {
            PurgeFailure::Message(message) => assert!(message.contains("4")),
            PurgeFailure::Auth => panic!("an empty failure is not auth"),
        }
    }

    #[test]
    fn only_package_installs_need_root() {
        let (_, needs_password) = system_items(InstallKind::Deb, None);
        assert!(needs_password);
        let (_, needs_password) = system_items(InstallKind::Rpm, None);
        assert!(needs_password);
        let (items, needs_password) = system_items(InstallKind::AppImage, Some(Path::new("/x")));
        assert!(!needs_password);
        assert_eq!(items, vec!["Delete /x"]);
        let (_, needs_password) = system_items(InstallKind::Source, None);
        assert!(!needs_password);
    }

    #[test]
    fn the_plan_matches_the_detected_install() {
        let plan = plan();
        assert_eq!(
            plan.package.is_some(),
            matches!(plan.kind, InstallKind::Deb | InstallKind::Rpm)
        );
        assert!(!plan.kind_label.is_empty());
        for item in &plan.data_items {
            assert!(
                std::path::Path::new(&item.path).exists(),
                "plan listed a path that is gone: {}",
                item.path
            );
        }
    }

    #[test]
    fn every_listed_path_goes_including_nested_ones() {
        let root = std::env::temp_dir().join(format!("knitt-remove-{}", std::process::id()));
        let nested = root.join("config").join("inner");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("settings.json"), b"{}").unwrap();
        let single = root.join("autostart.desktop");
        std::fs::write(&single, b"[Desktop Entry]").unwrap();
        let missing = root.join("never-existed");

        let listed = vec![
            (root.join("config"), "directory"),
            (single.clone(), "file"),
            (missing, "absent"),
        ];
        let (removed, failures) = remove_paths(&listed);

        assert_eq!(removed.len(), 2, "removed: {removed:?}");
        assert!(failures.is_empty(), "failures: {failures:?}");
        assert!(!root.join("config").exists());
        assert!(!single.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_symlink_is_unlinked_without_touching_its_target() {
        let root = std::env::temp_dir().join(format!("knitt-uninstall-{}", std::process::id()));
        let target = root.join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("keep"), b"x").unwrap();
        let link = root.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        remove_path(&link).unwrap();

        assert!(!link.exists());
        assert!(target.join("keep").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}

