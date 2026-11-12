use std::fs;
use std::path::{Path, PathBuf};

/// Escapes a path for a `Desktop Entry` `Exec=` value. Desktop entries are not
/// shell commands, so quoting is enough; nothing is ever passed to a shell.
fn desktop_exec(exe: &str) -> String {
    if exe
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._+-@".contains(c))
    {
        return exe.to_string();
    }
    let escaped = exe.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn autostart_file() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("autostart").join("knitt.desktop"))
}

/// The executable to launch at login. AppImages expose `APPIMAGE`; other
/// builds use the running binary.
fn launcher_path() -> Result<String, String> {
    if let Ok(appimage) = std::env::var("APPIMAGE")
        && !appimage.trim().is_empty()
    {
        return Ok(appimage);
    }
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .map_err(|_| "Could not determine the Knitt executable.".to_string())
}

fn set_enabled_in(dir: &Path, enabled: bool, exe: &str) -> Result<bool, String> {
    let path = dir.join("knitt.desktop");
    if enabled {
        fs::create_dir_all(dir)
            .map_err(|_| "Could not create the autostart folder.".to_string())?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Knitt\nComment=Git client\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            desktop_exec(exe)
        );
        fs::write(&path, content)
            .map_err(|_| "Could not enable launch at login.".to_string())?;
    } else {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Could not disable launch at login.".to_string()),
        }
    }
    Ok(path.exists())
}

fn is_enabled_in(dir: &Path) -> bool {
    dir.join("knitt.desktop").exists()
}

/// Whether the desktop-autostart entry exists. This reads the actual file, so
/// removing it outside Knitt is reflected here.
pub fn is_enabled() -> bool {
    autostart_file()
        .and_then(|path| path.parent().map(is_enabled_in))
        .unwrap_or(false)
}

pub fn set_enabled(enabled: bool) -> Result<bool, String> {
    let path =
        autostart_file().ok_or_else(|| "Could not find the autostart folder.".to_string())?;
    let dir = path
        .parent()
        .ok_or_else(|| "Could not find the autostart folder.".to_string())?;
    let exe = if enabled {
        launcher_path()?
    } else {
        String::new()
    };
    set_enabled_in(dir, enabled, &exe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-autostart-{}-{}",
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn enabling_writes_a_desktop_entry() {
        let dir = temp_dir();
        assert!(!is_enabled_in(&dir));

        assert!(set_enabled_in(&dir, true, "/opt/knitt/knitt").unwrap());
        let content = fs::read_to_string(dir.join("knitt.desktop")).unwrap();
        assert!(content.contains("Exec=/opt/knitt/knitt"));
        assert!(content.contains("Type=Application"));
        assert!(is_enabled_in(&dir));

        assert!(!set_enabled_in(&dir, false, "/opt/knitt/knitt").unwrap());
        assert!(!dir.join("knitt.desktop").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_with_spaces_are_quoted() {
        let dir = temp_dir();
        set_enabled_in(&dir, true, "/home/user/My Apps/knitt").unwrap();
        let content = fs::read_to_string(dir.join("knitt.desktop")).unwrap();
        assert!(content.contains("Exec=\"/home/user/My Apps/knitt\""));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disabling_missing_entry_is_not_an_error() {
        let dir = temp_dir();
        assert!(!set_enabled_in(&dir, false, "").unwrap());
        let _ = fs::remove_dir_all(&dir);
    }
}
