//! Removing Knitt itself: the package or AppImage first, then every file and
//! keyring entry the app created. The plan is read-only and shown before the
//! user confirms, so nothing is guessed at removal time.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// The password dialog was dismissed; nothing was changed.
    pub cancelled: bool,
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
    let (system_items, needs_password) = system_items(kind, appimage.as_deref());

    UninstallPlan {
        kind,
        kind_label: kind.label().to_string(),
        package: matches!(kind, InstallKind::Deb | InstallKind::Rpm).then(|| PACKAGE.to_string()),
        needs_password,
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
pub fn run() -> Result<UninstallOutcome, String> {
    let appimage = appimage_path();
    let kind = classify(
        appimage.as_deref(),
        running_from_appimage_mount(),
        deb_installed(),
        rpm_installed(),
    );
    let mut outcome = UninstallOutcome::default();

    // Package files live in /usr, so that step needs the password and runs
    // first: a dismissed dialog must leave everything untouched.
    if let Some((program, args)) = purge_command(kind) {
        match remove_package(program, args)? {
            Step::Cancelled => {
                outcome.cancelled = true;
                return Ok(outcome);
            }
            Step::Done => outcome.system_removed = true,
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
    let mut removed = Vec::new();
    let mut failures = Vec::new();

    for (path, _) in data_paths() {
        if !path.exists() {
            continue;
        }
        match remove_path(&path) {
            Ok(()) => removed.push(path.display().to_string()),
            Err(message) => failures.push(message),
        }
    }

    // Keyring entries live outside the filesystem.
    let secret = crate::auth::secret_store::SecretStore::new(KEYRING_APP);
    for account in saved_accounts() {
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

enum Step {
    Done,
    Cancelled,
}

/// Runs the package manager as root through polkit, which is what raises the
/// system password dialog for a desktop app. Falls back to a cached `sudo`
/// credential, then to an explanation the user can act on.
fn remove_package(program: &str, args: &[&str]) -> Result<Step, String> {
    let program = find_in_path(program)
        .ok_or_else(|| format!("{program} is not installed on this system."))?;

    if let Some(pkexec) = find_in_path("pkexec") {
        let output = Command::new(pkexec)
            .arg(&program)
            .args(args)
            .output()
            .map_err(|error| {
                tracing::warn!(%error, "could not run pkexec");
                "Could not open the password dialog.".to_string()
            })?;
        match output.status.code() {
            Some(0) => return Ok(Step::Done),
            // polkit uses 126 when the dialog was dismissed or authorization
            // was refused, which is a cancel rather than a failure.
            Some(126) => return Ok(Step::Cancelled),
            Some(code) => {
                let program = program.to_string_lossy().into_owned();
                return Err(package_error(&program, args, code, &output));
            }
            None => {
                return Err("The password dialog closed unexpectedly.".to_string());
            }
        }
    }

    // No polkit agent: a valid sudo timestamp still lets this work without a
    // terminal, so try it before giving up.
    let sudo_worked = Command::new("sudo")
        .arg("-n")
        .arg("--")
        .arg(&program)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success());
    if sudo_worked {
        return Ok(Step::Done);
    }

    Err(format!(
        "Knitt could not ask for your password. Remove the package from a terminal with `sudo {} {}`, then delete the folders listed above.",
        program.display(),
        args.join(" ")
    ))
}

fn package_error(program: &str, args: &[&str], code: i32, output: &std::process::Output) -> String {
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.lines().find(|line| !line.trim().is_empty());
    tracing::warn!(code, detail = ?detail, "package removal failed");
    match detail {
        Some(line) => format!("Could not remove the package: {}", line.trim()),
        None => format!(
            "Could not remove the package ({} {} exited with {code}).",
            program,
            args.join(" ")
        ),
    }
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
    fn only_package_installs_need_a_password() {
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
            plan.needs_password,
            matches!(plan.kind, InstallKind::Deb | InstallKind::Rpm)
        );
        assert_eq!(plan.package.is_some(), plan.needs_password);
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

