//! External application integrations: editor, terminal and file manager.
//!
//! Launches always use a direct argument vector. Nothing here is passed to a
//! shell, so a repository path can never be interpreted as shell syntax.

use crate::config::settings::Settings;
use serde::Serialize;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const TARGET_PLACEHOLDER: &str = "%TARGET_PATH%";
/// Placeholders of a diff or merge tool template. Each one is replaced by the
/// path of a file Knitt prepared for the tool.
pub const LOCAL_PLACEHOLDER: &str = "%LOCAL_PATH%";
pub const REMOTE_PLACEHOLDER: &str = "%REMOTE_PATH%";
pub const BASE_PLACEHOLDER: &str = "%BASE_PATH%";
pub const MERGED_PLACEHOLDER: &str = "%MERGED_PATH%";

const TOOL_PLACEHOLDERS: [&str; 4] = [
    LOCAL_PLACEHOLDER,
    REMOTE_PLACEHOLDER,
    BASE_PLACEHOLDER,
    MERGED_PLACEHOLDER,
];

/// `(id, display name, binary, argument template)`.
type Candidate = (&'static str, &'static str, &'static str, &'static str);

const EDITOR_CANDIDATES: &[Candidate] = &[
    ("vscode", "Visual Studio Code", "code", TARGET_PLACEHOLDER),
    ("vscodium", "VSCodium", "codium", TARGET_PLACEHOLDER),
    ("zed", "Zed", "zed", TARGET_PLACEHOLDER),
    ("sublime", "Sublime Text", "subl", TARGET_PLACEHOLDER),
    ("idea", "IntelliJ IDEA", "idea", TARGET_PLACEHOLDER),
    ("pycharm", "PyCharm", "pycharm", TARGET_PLACEHOLDER),
    ("webstorm", "WebStorm", "webstorm", TARGET_PLACEHOLDER),
    ("kate", "Kate", "kate", TARGET_PLACEHOLDER),
    (
        "gnome-text-editor",
        "GNOME Text Editor",
        "gnome-text-editor",
        TARGET_PLACEHOLDER,
    ),
];

/// Tools that compare two files. Every detected tool receives the two paths in
/// the order Knitt writes them: committed version, then working-tree version.
const DIFFTOOL_CANDIDATES: &[Candidate] = &[
    ("meld", "Meld", "meld", "%LOCAL_PATH% %REMOTE_PATH%"),
    ("kdiff3", "KDiff3", "kdiff3", "%LOCAL_PATH% %REMOTE_PATH%"),
    ("kompare", "Kompare", "kompare", "%LOCAL_PATH% %REMOTE_PATH%"),
    ("diffuse", "Diffuse", "diffuse", "%LOCAL_PATH% %REMOTE_PATH%"),
    ("tkdiff", "TkDiff", "tkdiff", "%LOCAL_PATH% %REMOTE_PATH%"),
    ("xxdiff", "xxdiff", "xxdiff", "%LOCAL_PATH% %REMOTE_PATH%"),
];

/// Tools that write a three-way merge. The templates follow the tools' own
/// conventions: the result flag, then local, base and remote.
const MERGETOOL_CANDIDATES: &[Candidate] = &[
    (
        "meld",
        "Meld",
        "meld",
        "--output %MERGED_PATH% %LOCAL_PATH% %BASE_PATH% %REMOTE_PATH%",
    ),
    (
        "kdiff3",
        "KDiff3",
        "kdiff3",
        "-o %MERGED_PATH% %LOCAL_PATH% %BASE_PATH% %REMOTE_PATH%",
    ),
];

const TERMINAL_CANDIDATES: &[Candidate] = &[
    (
        "gnome-terminal",
        "GNOME Terminal",
        "gnome-terminal",
        "--working-directory=%TARGET_PATH%",
    ),
    ("konsole", "Konsole", "konsole", "--workdir %TARGET_PATH%"),
    ("kitty", "Kitty", "kitty", "--directory %TARGET_PATH%"),
    (
        "alacritty",
        "Alacritty",
        "alacritty",
        "--working-directory %TARGET_PATH%",
    ),
    (
        "xfce4-terminal",
        "Xfce Terminal",
        "xfce4-terminal",
        "--working-directory=%TARGET_PATH%",
    ),
    (
        "tilix",
        "Tilix",
        "tilix",
        "--working-directory=%TARGET_PATH%",
    ),
    ("foot", "foot", "foot", "--working-directory=%TARGET_PATH%"),
    ("wezterm", "WezTerm", "wezterm", "start --cwd %TARGET_PATH%"),
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DetectedApp {
    pub id: String,
    pub name: String,
    /// Resolved absolute path, so the UI can show exactly what will run.
    pub path: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DetectedIntegrations {
    pub editors: Vec<DetectedApp>,
    pub terminals: Vec<DetectedApp>,
    pub difftools: Vec<DetectedApp>,
    pub mergetools: Vec<DetectedApp>,
}

/// Applications that are installed right now. Candidates that are missing
/// from `PATH` are simply omitted; the list is never a promise that an app
/// exists.
pub fn detect() -> DetectedIntegrations {
    DetectedIntegrations {
        editors: detect_group(EDITOR_CANDIDATES),
        terminals: detect_group(TERMINAL_CANDIDATES),
        difftools: detect_group(DIFFTOOL_CANDIDATES),
        mergetools: detect_group(MERGETOOL_CANDIDATES),
    }
}

fn detect_group(candidates: &[Candidate]) -> Vec<DetectedApp> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for &(id, name, binary, arguments) in candidates {
        let Some(path) = resolve_executable(binary) else {
            continue;
        };
        let path = path.to_string_lossy().to_string();
        if seen.insert(path.clone()) {
            apps.push(DetectedApp {
                id: id.to_string(),
                name: name.to_string(),
                path,
                arguments: arguments.to_string(),
            });
        }
    }
    apps
}

/// Resolves a configured executable. Accepts an absolute path, a path with a
/// separator, or a bare binary name looked up in `PATH`.
pub fn resolve_executable(value: &str) -> Option<PathBuf> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let path = Path::new(value);
    if path.is_absolute() || path.components().count() > 1 {
        return executable_path(path);
    }
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|dir| executable_path(&dir.join(value)))
}

fn executable_path(path: &Path) -> Option<PathBuf> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    std::fs::canonicalize(path).ok()
}

/// Returned when an argument template has an unterminated quote or a trailing
/// escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgumentError;

/// Replaces every `(placeholder, value)` pair in every argument. The flag
/// reports whether at least one placeholder was found, so callers can decide
/// whether a template without placeholders still makes sense.
fn substitute(
    template: &str,
    values: &[(&str, &str)],
) -> Result<(Vec<OsString>, bool), ArgumentError> {
    let mut arguments = Vec::new();
    let mut substituted = false;
    for token in split_arguments(template)? {
        let mut token = token;
        for (placeholder, value) in values {
            if token.contains(placeholder) {
                substituted = true;
                token = token.replace(placeholder, value);
            }
        }
        arguments.push(OsString::from(token));
    }
    Ok((arguments, substituted))
}

/// Substitutes `%TARGET_PATH%` in every argument. When the template does not
/// mention the placeholder at all, the target is appended as one argument so
/// plain commands like an editor binary still open the folder.
pub fn build_arguments(template: &str, target: &Path) -> Result<Vec<OsString>, ArgumentError> {
    let target_text = target.to_string_lossy();
    let (mut arguments, substituted) =
        substitute(template, &[(TARGET_PLACEHOLDER, target_text.as_ref())])?;
    if !substituted {
        arguments.push(target.as_os_str().to_os_string());
    }
    Ok(arguments)
}

/// Substitutes the file placeholders of a diff or merge tool.
///
/// Unlike [`build_arguments`], a template that mentions none of the
/// placeholders is rejected: the tool would start without the files Knitt just
/// prepared. So is a template that still names a placeholder the caller did
/// not supply.
pub fn build_tool_arguments(
    template: &str,
    values: &[(&str, &str)],
) -> Result<Vec<OsString>, String> {
    let (arguments, substituted) = substitute(template, values)
        .map_err(|_| "The argument template has an unmatched quote.".to_string())?;
    if !substituted {
        return Err(
            "The argument template must include a file placeholder such as %LOCAL_PATH%."
                .to_string(),
        );
    }
    let unused = arguments.iter().any(|argument| {
        let text = argument.to_string_lossy();
        TOOL_PLACEHOLDERS
            .iter()
            .chain(std::iter::once(&TARGET_PLACEHOLDER))
            .any(|placeholder| text.contains(placeholder))
    });
    if unused {
        return Err(
            "The argument template uses a placeholder this tool does not provide.".to_string(),
        );
    }
    Ok(arguments)
}

/// Splits an argument template into an argument vector. Supports single and
/// double quotes and backslash escapes outside single quotes. There is no
/// variable expansion, globbing or command substitution.
fn split_arguments(input: &str) -> Result<Vec<String>, ArgumentError> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut has_token = false;

    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            has_token = true;
            continue;
        }
        match quote {
            Some(quote_character) => {
                if character == quote_character {
                    quote = None;
                } else if character == '\\' && quote_character == '"' {
                    escaped = true;
                } else {
                    current.push(character);
                }
                has_token = true;
            }
            None => match character {
                '\'' | '"' => {
                    quote = Some(character);
                    has_token = true;
                }
                '\\' => {
                    escaped = true;
                    has_token = true;
                }
                c if c.is_whitespace() => {
                    if has_token {
                        arguments.push(std::mem::take(&mut current));
                        has_token = false;
                    }
                }
                c => {
                    current.push(c);
                    has_token = true;
                }
            },
        }
    }

    if escaped || quote.is_some() {
        return Err(ArgumentError);
    }
    if has_token {
        arguments.push(current);
    }
    Ok(arguments)
}

fn launch_error(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::NotFound => "The selected application could not be found.".to_string(),
        std::io::ErrorKind::PermissionDenied => {
            "Knitt is not allowed to run the selected application.".to_string()
        }
        _ => "The selected application could not be started.".to_string(),
    }
}

fn missing_executable() -> String {
    "The selected application could not be found. Choose it again in Settings.".to_string()
}

/// Starts an application and returns immediately. Errors are user-facing and
/// do not include internal paths or OS error strings.
pub fn launch(executable: &str, arguments: &str, target: &Path) -> Result<(), String> {
    let arguments = build_arguments(arguments, target)
        .map_err(|_| "The argument template has an unmatched quote.".to_string())?;
    launch_tool(executable, &arguments)
}

/// Starts a tool with an already prepared argument vector. Used by the diff and
/// merge integrations, whose files live in a scratch directory.
pub fn launch_tool(executable: &str, arguments: &[OsString]) -> Result<(), String> {
    let executable = resolve_executable(executable).ok_or_else(missing_executable)?;
    Command::new(&executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|error| {
            log::warn!("integration launch failed: {error}");
            launch_error(&error)
        })
}

pub fn existing_directory(target: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(target.trim());
    if path.is_dir() {
        Ok(path)
    } else {
        Err("That folder is no longer available.".to_string())
    }
}

/// Resolves an existing file. Used to open a changed or conflicted file in the
/// configured editor; folders and missing paths are refused.
pub fn existing_file(target: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(target.trim());
    match std::fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => Ok(path),
        _ => Err("That file is no longer available.".to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Editor,
    Terminal,
}

impl Kind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "editor" => Some(Self::Editor),
            "terminal" => Some(Self::Terminal),
            _ => None,
        }
    }
}

/// Launches the saved editor or terminal for a directory.
pub fn open(kind: Kind, target: &str) -> Result<(), String> {
    let target = existing_directory(target)?;
    let settings = Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    let config = match kind {
        Kind::Editor => settings.external_editor,
        Kind::Terminal => settings.terminal,
    };
    let executable = config
        .executable
        .filter(|value| !value.trim().is_empty())
        .ok_or(match kind {
            Kind::Editor => {
                "No external editor is configured. Choose one in Settings → Integrations."
            }
            Kind::Terminal => "No terminal is configured. Choose one in Settings → Integrations.",
        })?;
    launch(&executable, &config.arguments, &target)
}

/// Opens a single existing file in the configured external editor. Used for
/// changed files and for conflicted files while an operation is in progress.
pub fn open_file(target: &str) -> Result<(), String> {
    let path = existing_file(target)?;
    let settings = Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    let config = settings.external_editor;
    let executable = config
        .executable
        .filter(|value| !value.trim().is_empty())
        .ok_or("No external editor is configured. Choose one in Settings → Integrations.")?;
    launch(&executable, &config.arguments, &path)
}

/// Opens a directory with the desktop's registered file manager. `xdg-open`
/// delegates to whatever the environment provides, so GNOME Files is not
/// assumed.
pub fn open_file_manager(target: &str) -> Result<(), String> {
    let target = existing_directory(target)?;
    Command::new("xdg-open")
        .arg(&target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|_| "No file manager is available to open that folder.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("knitt-integrations-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn arguments_split_on_whitespace() {
        assert_eq!(
            split_arguments("--workdir %TARGET_PATH% -x").unwrap(),
            vec!["--workdir", "%TARGET_PATH%", "-x"]
        );
    }

    #[test]
    fn arguments_honour_quotes_and_escapes() {
        assert_eq!(
            split_arguments(r#"--title "My Repo" --path '/a b'"#).unwrap(),
            vec!["--title", "My Repo", "--path", "/a b"]
        );
        assert_eq!(
            split_arguments(r#"one\ two "quoted \"value\"""#).unwrap(),
            vec!["one two", "quoted \"value\""]
        );
        assert_eq!(split_arguments("''").unwrap(), vec![""]);
    }

    #[test]
    fn unmatched_quote_is_rejected() {
        assert!(split_arguments("\"open").is_err());
        assert!(split_arguments("trailing\\").is_err());
    }

    #[test]
    fn placeholder_is_substituted_in_place() {
        let target = Path::new("/tmp/knitt repo");
        let arguments = build_arguments("--workdir %TARGET_PATH% --new-window", target).unwrap();
        assert_eq!(
            arguments,
            vec![
                OsString::from("--workdir"),
                OsString::from("/tmp/knitt repo"),
                OsString::from("--new-window"),
            ]
        );
    }

    #[test]
    fn target_is_appended_when_placeholder_is_absent() {
        let target = Path::new("/tmp/knitt-repo");
        let arguments = build_arguments("--new-window", target).unwrap();
        assert_eq!(
            arguments,
            vec![
                OsString::from("--new-window"),
                OsString::from("/tmp/knitt-repo")
            ]
        );
    }

    #[test]
    fn tool_arguments_replace_every_placeholder() {
        let arguments = build_tool_arguments(
            "--output %MERGED_PATH% %LOCAL_PATH% %BASE_PATH% %REMOTE_PATH%",
            &[
                (LOCAL_PLACEHOLDER, "/tmp/local/a.txt"),
                (BASE_PLACEHOLDER, "/tmp/base/a.txt"),
                (REMOTE_PLACEHOLDER, "/tmp/remote/a.txt"),
                (MERGED_PLACEHOLDER, "/tmp/repo/a.txt"),
            ],
        )
        .unwrap();
        assert_eq!(
            arguments,
            vec![
                OsString::from("--output"),
                OsString::from("/tmp/repo/a.txt"),
                OsString::from("/tmp/local/a.txt"),
                OsString::from("/tmp/base/a.txt"),
                OsString::from("/tmp/remote/a.txt"),
            ]
        );
    }

    #[test]
    fn tool_arguments_require_a_placeholder() {
        assert_eq!(
            build_tool_arguments("--wait", &[(LOCAL_PLACEHOLDER, "/tmp/a")]).unwrap_err(),
            "The argument template must include a file placeholder such as %LOCAL_PATH%."
        );
        assert_eq!(
            build_tool_arguments("\"open", &[(LOCAL_PLACEHOLDER, "/tmp/a")]).unwrap_err(),
            "The argument template has an unmatched quote."
        );
    }

    #[test]
    fn tool_arguments_refuse_placeholders_that_were_not_supplied() {
        for template in ["%LOCAL_PATH% %BASE_PATH%", "%LOCAL_PATH% %TARGET_PATH%"] {
            assert_eq!(
                build_tool_arguments(template, &[(LOCAL_PLACEHOLDER, "/tmp/a")]).unwrap_err(),
                "The argument template uses a placeholder this tool does not provide.",
                "{template} should be refused"
            );
        }
    }

    #[test]
    fn resolver_requires_an_executable_file() {
        let dir = temp_dir();
        let plain = dir.join("plain");
        std::fs::write(&plain, "#!/bin/sh\n").unwrap();
        assert_eq!(resolve_executable(plain.to_str().unwrap()), None);
        assert_eq!(resolve_executable(dir.to_str().unwrap()), None);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(resolve_executable(plain.to_str().unwrap()).is_some());
        }
    }

    #[test]
    fn existing_file_accepts_files_only() {
        let dir = temp_dir();
        let file = dir.join("a.txt");
        std::fs::write(&file, "a\n").unwrap();
        assert_eq!(
            existing_file(file.to_str().unwrap()).unwrap(),
            file
        );
        assert_eq!(
            existing_file(dir.to_str().unwrap()).unwrap_err(),
            "That file is no longer available."
        );
        assert_eq!(
            existing_file(dir.join("missing.txt").to_str().unwrap()).unwrap_err(),
            "That file is no longer available."
        );
    }

    #[test]
    fn missing_executable_is_reported_without_details() {
        let error = launch("/nonexistent/knitt-editor", "", Path::new("/tmp")).unwrap_err();
        assert_eq!(
            error,
            "The selected application could not be found. Choose it again in Settings."
        );
    }

    #[test]
    fn detection_does_not_panic() {
        let detected = detect();
        for app in detected
            .editors
            .iter()
            .chain(detected.terminals.iter())
            .chain(detected.difftools.iter())
            .chain(detected.mergetools.iter())
        {
            assert!(!app.name.is_empty());
            assert!(Path::new(&app.path).is_absolute());
        }
    }
}
