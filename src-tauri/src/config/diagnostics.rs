use crate::auth::accounts::AccountsStore;
use crate::config::paths;
use crate::config::settings::Settings;
use crate::utils::logging;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Non-secret environment details shown in Settings → Advanced.
///
/// Deliberately excludes repository paths, tokens, environment variables and
/// file contents.
#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentInfo {
    pub app_version: String,
    pub git_backend: String,
    pub os: String,
    pub arch: String,
    pub desktop: Option<String>,
    pub settings_dir: String,
    pub data_dir: String,
    pub logs_dir: String,
    pub log_size: u64,
    pub app_data_size: u64,
    pub account_count: usize,
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                return 0;
            };
            if meta.file_type().is_symlink() {
                return 0;
            }
            if meta.is_dir() {
                dir_size(&path)
            } else {
                meta.len()
            }
        })
        .sum()
}

fn display_dir(dir: Option<PathBuf>) -> String {
    dir.map(|path| path.display().to_string())
        .unwrap_or_else(|| "Unavailable".to_string())
}

fn log_size() -> u64 {
    logging::log_path()
        .ok()
        .and_then(|path| fs::metadata(path).ok())
        .map(|meta| meta.len())
        .unwrap_or(0)
}

pub fn environment() -> EnvironmentInfo {
    let version = git2::Version::get();
    let (major, minor, rev) = version.libgit2_version();
    let settings_dir = paths::config_dir().ok();
    let data_dir = paths::data_dir().ok();
    let logs_dir = logging::log_dir().ok();

    let app_data_size = [&settings_dir, &data_dir, &logs_dir]
        .into_iter()
        .flatten()
        .map(|dir| dir_size(dir))
        .sum();

    EnvironmentInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        git_backend: format!("libgit2 {major}.{minor}.{rev}"),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        desktop: std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("XDG_SESSION_DESKTOP"))
            .ok()
            .filter(|value| !value.trim().is_empty()),
        settings_dir: display_dir(settings_dir),
        data_dir: display_dir(data_dir),
        logs_dir: display_dir(logs_dir),
        log_size: log_size(),
        app_data_size,
        account_count: AccountsStore::load()
            .map(|store| store.accounts.len())
            .unwrap_or(0),
    }
}

fn integration_state(label: &str, configured: bool) -> String {
    if configured {
        format!("- {label}: configured")
    } else {
        format!("- {label}: not configured")
    }
}

/// A plain-text report the user can read before sharing. Contains no tokens,
/// repository paths, diffs or environment variables.
pub fn report() -> String {
    let info = environment();
    let settings = Settings::load().unwrap_or_default();

    let mut out = String::new();
    out.push_str("Knitt diagnostic report\n");
    out.push_str(&format!(
        "Generated: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S %z")
    ));

    out.push_str("Application\n");
    out.push_str(&format!("- Knitt {}\n", info.app_version));
    out.push_str(&format!("- Git backend: {}\n", info.git_backend));
    out.push_str(&format!("- Platform: {} {}\n", info.os, info.arch));
    match &info.desktop {
        Some(desktop) => out.push_str(&format!("- Desktop: {desktop}\n")),
        None => out.push_str("- Desktop: unknown\n"),
    }

    out.push_str("\nStorage\n");
    out.push_str(&format!("- Settings: {}\n", info.settings_dir));
    out.push_str(&format!("- App data: {}\n", info.data_dir));
    out.push_str(&format!("- Logs: {} ({} bytes)\n", info.logs_dir, info.log_size));
    out.push_str(&format!(
        "- Total app data: {} bytes\n",
        info.app_data_size
    ));

    out.push_str("\nSettings (non-secret)\n");
    out.push_str(&format!("- Theme: {}\n", settings.theme));
    out.push_str(&format!("- Default branch: {}\n", settings.default_branch));
    out.push_str(&format!(
        "- Restore last repository: {}\n",
        settings.restore_last_repository
    ));
    out.push_str(&format!(
        "- Default repository directory set: {}\n",
        settings.default_repository_directory.is_some()
    ));
    out.push_str(&format!("- Diff tab width: {}\n", settings.diff_tab_width));
    out.push_str(&integration_state(
        "External editor",
        settings
            .external_editor
            .executable
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
    ));
    out.push_str(&integration_state(
        "Terminal",
        settings
            .terminal
            .executable
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
    ));

    out.push_str("\nAccounts\n");
    out.push_str(&format!(
        "- {} GitHub account(s) configured\n",
        info.account_count
    ));

    out.push_str("\nNo credentials, repository paths or file contents are included.\n");
    out
}

/// Removes the current and rotated log file. Settings and accounts are
/// untouched.
pub fn clear_logs() -> Result<(), String> {
    let dir = logging::log_dir().map_err(|_| "Could not find the log folder.".to_string())?;
    for name in ["knitt.log", "knitt.log.1"] {
        match fs::remove_file(dir.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Could not clear the logs.".to_string()),
        }
    }
    Ok(())
}

/// Writes the report to a user-chosen path. The path comes from the native save
/// dialog; the report itself is generated here so no arbitrary content can be
/// written through this command.
pub fn export(path: &str) -> Result<(), String> {
    let path = Path::new(path.trim());
    if path.as_os_str().is_empty() || path.is_dir() {
        return Err("Choose a file to save the report.".to_string());
    }
    fs::write(path, report()).map_err(|_| "Could not save the diagnostic report.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-diagnostics-{}-{}",
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn dir_size_counts_files_and_skips_symlinks() {
        let dir = temp_dir();
        fs::write(dir.join("a.txt"), b"12345").unwrap();
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("nested/b.txt"), b"123").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", dir.join("link")).unwrap();

        assert_eq!(dir_size(&dir), 8);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn report_contains_no_secret_markers() {
        let report = report();
        assert!(report.starts_with("Knitt diagnostic report"));
        assert!(report.contains("Git backend: libgit2"));
        for marker in ["gho_", "ghp_", "token", "Authorization"] {
            assert!(
                !report.to_lowercase().contains(&marker.to_lowercase()),
                "report leaked {marker}"
            );
        }
    }

    #[test]
    fn export_writes_generated_report() {
        let dir = temp_dir();
        let path = dir.join("report.txt");
        export(path.to_str().unwrap()).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(written, report());
        assert!(export("").is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
