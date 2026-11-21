use crate::config::paths;
use crate::config::settings::Settings;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Keep the crash folder small: the newest reports win.
pub const MAX_REPORTS: usize = 10;

/// A panic message can embed a lot of text; keep the report readable.
const MAX_MESSAGE_CHARS: usize = 4_000;
const MAX_READ_BYTES: u64 = 262_144;
const FILE_PREFIX: &str = "crash-";
const FILE_SUFFIX: &str = ".txt";

/// One file in the crash folder, as listed in Settings → Advanced.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CrashReport {
    /// File name only; never a path.
    pub file: String,
    /// First panic line, for the list.
    pub summary: String,
    /// Milliseconds since the Unix epoch.
    pub time: u64,
    pub size: u64,
}

pub fn dir() -> io::Result<PathBuf> {
    let dir = paths::data_dir()?.join("crashes");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Installs a panic hook that saves a redacted report when crash reporting is
/// enabled, then defers to the hook that was installed before (which prints to
/// stderr). Reporting never changes the panic behaviour itself.
pub fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // A panic inside a panic hook aborts the process, so record defensively.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| record(info)));
        previous(info);
    }));
}

fn record(info: &std::panic::PanicHookInfo<'_>) {
    let enabled = Settings::load()
        .map(|settings| settings.crash_reporting)
        .unwrap_or(false);
    if !enabled {
        return;
    }
    let Ok(dir) = dir() else {
        return;
    };
    let message = payload_message(info.payload());
    let location = info
        .location()
        .map(|location| format!("{}:{}:{}", location.file(), location.line(), location.column()));
    let _ = write_when_enabled(&dir, &message, location.as_deref(), enabled, Utc::now());
}

/// The opt-in check lives here so it can be exercised without a real panic.
fn write_when_enabled(
    dir: &Path,
    message: &str,
    location: Option<&str>,
    enabled: bool,
    now: DateTime<Utc>,
) -> Option<PathBuf> {
    if !enabled {
        return None;
    }
    write_to(dir, message, location, now).ok()
}

fn payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "unknown panic".to_string()
}

/// Report text for a panic. Paths under the user's home directory and Knitt's
/// own storage folders are shortened, so a report can be shared as-is.
pub fn render_report(message: &str, location: Option<&str>, now: DateTime<Utc>) -> String {
    let mut message = redact(message);
    if message.chars().count() > MAX_MESSAGE_CHARS {
        message = message.chars().take(MAX_MESSAGE_CHARS).collect();
        message.push('…');
    }
    let mut out = String::new();
    out.push_str("Knitt crash report\n");
    out.push_str(&format!("Version: {}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!(
        "Platform: {} {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    out.push_str(&format!("Time: {}\n", now.format("%Y-%m-%d %H:%M:%S UTC")));
    out.push_str(&format!("Panic: {}\n", message.trim()));
    match location {
        Some(location) => out.push_str(&format!("Location: {}\n", redact(location))),
        None => out.push_str("Location: unknown\n"),
    }
    out.push_str(
        "\nSaved because crash reports are on. Repository contents and saved \
         credentials are never included; paths under your home folder are shortened.\n",
    );
    out
}

/// Replaces Knitt's storage folders and the home directory with placeholders.
pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    let folders: [(Option<PathBuf>, &str); 3] = [
        (paths::config_dir().ok(), "<settings>"),
        (paths::data_dir().ok(), "<app-data>"),
        (paths::state_dir().ok(), "<app-state>"),
    ];
    for (path, placeholder) in folders {
        if let Some(path) = path {
            let display = path.display().to_string();
            if !display.is_empty() {
                out = out.replace(&display, placeholder);
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let display = home.display().to_string();
        if display.len() > 1 {
            out = out.replace(&display, "~");
        }
    }
    out
}

fn write_to(
    dir: &Path,
    message: &str,
    location: Option<&str>,
    now: DateTime<Utc>,
) -> io::Result<PathBuf> {
    let path = unique_path(dir, now);
    fs::create_dir_all(dir)?;
    let report = render_report(message, location, now);
    fs::write(&path, report)?;
    restrict_permissions(&path)?;
    prune(dir);
    Ok(path)
}

fn unique_path(dir: &Path, now: DateTime<Utc>) -> PathBuf {
    let stamp = format!(
        "{}-{:03}",
        now.format("%Y%m%d-%H%M%S"),
        now.timestamp_subsec_millis()
    );
    let mut path = dir.join(format!("{FILE_PREFIX}{stamp}{FILE_SUFFIX}"));
    let mut attempt = 1;
    while path.exists() {
        path = dir.join(format!("{FILE_PREFIX}{stamp}-{attempt}{FILE_SUFFIX}"));
        attempt += 1;
    }
    path
}

/// Drops the oldest reports beyond [`MAX_REPORTS`]. File names carry the
/// timestamp, so name order is age order.
fn prune(dir: &Path) {
    let mut files: Vec<PathBuf> = report_files(dir);
    files.sort();
    if files.len() <= MAX_REPORTS {
        return;
    }
    for path in &files[..files.len() - MAX_REPORTS] {
        let _ = fs::remove_file(path);
    }
}

fn report_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_report_name(&file_name(path)))
        .collect()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn is_report_name(name: &str) -> bool {
    name.starts_with(FILE_PREFIX) && name.ends_with(FILE_SUFFIX)
}

/// Reports newest first. Unreadable entries are skipped instead of failing.
pub fn list() -> Vec<CrashReport> {
    let Ok(dir) = dir() else {
        return Vec::new();
    };
    list_from(&dir)
}

fn list_from(dir: &Path) -> Vec<CrashReport> {
    let mut paths = report_files(dir);
    paths.sort();
    paths
        .into_iter()
        .rev()
        .filter_map(|path| {
            let meta = fs::metadata(&path).ok()?;
            let text = read_limited(&path)?;
            Some(CrashReport {
                file: file_name(&path),
                summary: summary(&text),
                time: meta
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis() as u64)
                    .unwrap_or(0),
                size: meta.len(),
            })
        })
        .collect()
}

fn read_limited(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    let end = data.len().min(MAX_READ_BYTES as usize);
    Some(String::from_utf8_lossy(&data[..end]).to_string())
}

fn summary(text: &str) -> String {
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix("Panic: "))
        .unwrap_or("Crash report");
    let mut summary: String = line.chars().take(120).collect();
    if line.chars().count() > 120 {
        summary.push('…');
    }
    summary
}

/// Rejects anything that is not a bare report file name, so a command cannot
/// read or delete files outside the crash folder.
fn validated_file(file: &str) -> Result<String, String> {
    let name = file.trim();
    if !is_report_name(name) || name.contains(['/', '\\']) || name.contains("..") {
        return Err("Choose a crash report from the list.".to_string());
    }
    Ok(name.to_string())
}

pub fn read(file: &str) -> Result<String, String> {
    let name = validated_file(file)?;
    let path = dir()
        .map_err(|_| "Could not find the crash reports.".to_string())?
        .join(name);
    read_limited(&path).ok_or_else(|| "Could not read that crash report.".to_string())
}

/// Copies one report to a path chosen in the native save dialog.
pub fn export(path: &str, file: &str) -> Result<(), String> {
    let text = read(file)?;
    let target = Path::new(path.trim());
    if target.as_os_str().is_empty() || target.is_dir() {
        return Err("Choose a file to save the report.".to_string());
    }
    fs::write(target, text).map_err(|_| "Could not save the crash report.".to_string())
}

/// Removes saved reports only; settings, accounts and logs are untouched.
pub fn clear() -> Result<(), String> {
    let dir = dir().map_err(|_| "Could not find the crash reports.".to_string())?;
    for path in report_files(&dir) {
        fs::remove_file(&path).map_err(|_| "Could not delete the crash reports.".to_string())?;
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-crash-{}-{}",
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn at(day: u32, millis: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
            + chrono::Duration::milliseconds(millis as i64)
    }

    #[test]
    fn report_names_the_panic_and_platform() {
        let text = render_report("index out of bounds", Some("src/git/sync.rs:12:5"), at(1, 0));
        assert!(text.starts_with("Knitt crash report\n"));
        assert!(text.contains("Panic: index out of bounds\n"));
        assert!(text.contains("Location: src/git/sync.rs:12:5\n"));
        assert!(text.contains(&format!("Version: {}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn home_paths_are_shortened() {
        let home = dirs::home_dir().expect("home directory");
        let message = format!("open {} failed", home.join("projects/secret/notes.txt").display());
        let text = render_report(&message, None, at(1, 0));
        assert!(!text.contains(&home.display().to_string()));
        assert!(text.contains("~/projects/secret/notes.txt"));
    }

    #[test]
    fn long_messages_are_capped() {
        let text = render_report(&"x".repeat(MAX_MESSAGE_CHARS + 500), None, at(1, 0));
        assert!(text.contains('…'));
        assert!(text.len() < MAX_MESSAGE_CHARS + 1_000);
    }

    #[test]
    fn writes_one_file_and_restricts_it() {
        let dir = temp_dir();
        let path = write_to(&dir, "boom", Some("src/lib.rs:1:1"), at(2, 10)).unwrap();
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "crash-20260102-120000-010.txt");
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("Panic: boom"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_only_the_newest_reports() {
        let dir = temp_dir();
        for day in 1..=MAX_REPORTS as u32 + 2 {
            write_to(&dir, "boom", None, at(day, 0)).unwrap();
        }
        let kept = report_files(&dir);
        assert_eq!(kept.len(), MAX_REPORTS);
        // Day 1 and day 2 were pruned; the newest day survives.
        assert!(!kept.iter().any(|path| file_name(path) == "crash-20260101-120000-000.txt"));
        assert!(kept.iter().any(|path| file_name(path).starts_with("crash-20260112-")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_is_newest_first_and_summarises() {
        let dir = temp_dir();
        write_to(&dir, "old failure", None, at(1, 0)).unwrap();
        write_to(&dir, "new failure", None, at(3, 0)).unwrap();

        let reports = list_from(&dir);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].summary, "new failure");
        assert_eq!(reports[1].summary, "old failure");
        assert!(reports[0].size > 0);
        assert!(is_report_name(&reports[0].file));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disabled_reporting_writes_nothing() {
        let dir = temp_dir();
        assert!(write_when_enabled(&dir, "boom", None, false, at(1, 0)).is_none());
        assert!(report_files(&dir).is_empty());

        assert!(write_when_enabled(&dir, "boom", None, true, at(1, 0)).is_some());
        assert_eq!(report_files(&dir).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn report_names_are_validated() {
        assert!(validated_file("crash-20260101-120000-000.txt").is_ok());
        for bad in ["", "../settings.json", "crash-../../x.txt", "/etc/passwd", "notes.txt"] {
            assert!(validated_file(bad).is_err(), "{bad} was accepted");
        }
    }

    #[test]
    fn missing_report_reads_as_error() {
        assert!(read("crash-19990101-000000-000.txt").is_err());
    }
}
