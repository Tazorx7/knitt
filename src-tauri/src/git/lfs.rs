//! Git LFS, driven through the official `git-lfs` command.
//!
//! libgit2 has no LFS support, so these operations spawn the CLI with an
//! argument vector. Nothing is passed to a shell, and the repository path is
//! only used as the working directory. Hooks and filters stay in charge of
//! checkouts and commits; Knitt only manages patterns and moves objects.

use crate::integrations;
use git2::Repository;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BINARY: &str = "git-lfs";

/// Whether the CLI is installed, and which version.
#[derive(Debug, Clone, Serialize)]
pub struct LfsStatus {
    pub installed: bool,
    pub version: Option<String>,
}

/// One pattern from an attributes file, as reported by `git lfs track`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TrackedPattern {
    pub pattern: String,
    /// Attributes file the pattern came from, usually `.gitattributes`.
    pub source: String,
}

fn binary() -> Result<PathBuf, String> {
    integrations::resolve_executable(BINARY)
        .ok_or_else(|| "Git LFS is not installed. Install git-lfs to track large files.".to_string())
}

/// Resolves the CLI and reports its version. Never runs the tool against a
/// repository, so it is safe to call while a repository is opening.
pub fn status() -> LfsStatus {
    match binary() {
        Ok(executable) => status_of(&executable),
        Err(_) => LfsStatus {
            installed: false,
            version: None,
        },
    }
}

fn status_of(executable: &Path) -> LfsStatus {
    match run(executable, None, &["version"]) {
        Ok(output) => LfsStatus {
            installed: true,
            version: output
                .lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string),
        },
        Err(_) => LfsStatus {
            installed: false,
            version: None,
        },
    }
}

pub fn tracked(path: &str) -> Result<Vec<TrackedPattern>, String> {
    let executable = binary()?;
    tracked_at(&executable, path)
}

fn tracked_at(executable: &Path, path: &str) -> Result<Vec<TrackedPattern>, String> {
    let dir = workdir(path)?;
    let output = run(
        executable,
        Some(&dir),
        &["track"],
    )
    .map_err(|_| "Git LFS could not list the tracked patterns.".to_string())?;
    Ok(parse_tracked(&output))
}

/// Adds a pattern to `.gitattributes`. The edit then shows up as a normal
/// change, so the user decides when it is committed.
pub fn track(path: &str, pattern: &str) -> Result<(), String> {
    let pattern = check_pattern(pattern)?;
    let executable = binary()?;
    track_at(&executable, path, pattern)
}

fn track_at(executable: &Path, path: &str, pattern: &str) -> Result<(), String> {
    let dir = workdir(path)?;
    run(executable, Some(&dir), &["track", pattern])
        .map_err(|_| "Git LFS could not track that pattern.".to_string())?;
    Ok(())
}

/// Removes a pattern from the attributes file. Files already stored as
/// pointers stay pointers; only future additions follow the new rule.
pub fn untrack(path: &str, pattern: &str) -> Result<(), String> {
    let pattern = check_pattern(pattern)?;
    let executable = binary()?;
    untrack_at(&executable, path, pattern)
}

fn untrack_at(executable: &Path, path: &str, pattern: &str) -> Result<(), String> {
    let dir = workdir(path)?;
    run(executable, Some(&dir), &["untrack", pattern])
        .map_err(|_| "Git LFS could not untrack that pattern.".to_string())?;
    Ok(())
}

/// Downloads the objects a clone is missing.
pub fn pull(path: &str) -> Result<(), String> {
    let executable = binary()?;
    pull_at(&executable, path)
}

fn pull_at(executable: &Path, path: &str) -> Result<(), String> {
    let dir = workdir(path)?;
    run(executable, Some(&dir), &["pull"])
        .map_err(|_| "Could not download the LFS objects.".to_string())?;
    Ok(())
}

/// Uploads every object this branch references. Knitt pushes with libgit2,
/// which does not run Git hooks, so the upload is not automatic.
pub fn push(path: &str) -> Result<(), String> {
    let executable = binary()?;
    push_at(&executable, path)
}

fn push_at(executable: &Path, path: &str) -> Result<(), String> {
    let dir = workdir(path)?;
    let repo = Repository::open(&dir).map_err(|_| "Could not open the repository.".to_string())?;
    let remote = remote_name(&repo)?;
    run(executable, Some(&dir), &["push", "--all", &remote])
        .map_err(|_| "Could not upload the LFS objects.".to_string())?;
    Ok(())
}

/// The remote that receives LFS objects: the current branch's upstream, then
/// `origin`, then the only configured remote.
fn remote_name(repo: &Repository) -> Result<String, String> {
    if let Ok(head) = repo.head()
        && let Some(name) = head.name()
        && let Ok(remote) = repo.branch_upstream_remote(name)
        && let Some(remote) = remote.as_str()
    {
        return Ok(remote.to_string());
    }
    if repo.find_remote("origin").is_ok() {
        return Ok("origin".to_string());
    }
    let names: Vec<String> = repo
        .remotes()
        .map(|names| names.iter().flatten().map(str::to_string).collect())
        .unwrap_or_default();
    match names.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err("Add a remote before uploading LFS objects.".to_string()),
        _ => Err(
            "This repository has several remotes. Set an upstream branch so Knitt knows where to upload."
                .to_string(),
        ),
    }
}

fn check_pattern(pattern: &str) -> Result<&str, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("Enter a pattern such as *.psd.".to_string());
    }
    if pattern.chars().any(char::is_whitespace) {
        return Err("A pattern cannot contain spaces.".to_string());
    }
    if pattern.starts_with('-') {
        return Err("A pattern cannot start with a dash.".to_string());
    }
    Ok(pattern)
}

fn workdir(path: &str) -> Result<PathBuf, String> {
    Repository::open(path)
        .map_err(|_| "Could not open the repository.".to_string())?
        .workdir()
        .map(Path::to_path_buf)
        .ok_or_else(|| "This repository has no working tree.".to_string())
}

/// Runs the CLI and returns its standard output. Failures are logged in full
/// and reported without the tool's own wording or paths.
fn run(executable: &Path, dir: Option<&Path>, arguments: &[&str]) -> Result<String, String> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            log::warn!("git-lfs could not be started: {error}");
            "Git LFS could not be started.".to_string()
        })?;
    if !output.status.success() {
        log::warn!(
            "git-lfs {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Err("Git LFS could not complete that action.".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Reads the "Listing tracked patterns" block. Excluded patterns are ignored:
/// they describe what LFS must not store, which Knitt does not manage.
fn parse_tracked(output: &str) -> Vec<TrackedPattern> {
    let mut patterns = Vec::new();
    let mut in_tracked = false;
    for line in output.lines() {
        let line = line.trim();
        if line.eq_ignore_ascii_case("Listing tracked patterns") {
            in_tracked = true;
            continue;
        }
        if line.to_ascii_lowercase().starts_with("listing ") {
            in_tracked = false;
            continue;
        }
        if !in_tracked || line.is_empty() {
            continue;
        }
        let Some((pattern, source)) = line.rsplit_once(" (") else {
            continue;
        };
        let pattern = pattern.trim();
        if pattern.is_empty() {
            continue;
        }
        patterns.push(TrackedPattern {
            pattern: pattern.to_string(),
            source: source.trim_end_matches(')').to_string(),
        });
    }
    patterns
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-lfs-{}-{}-{}",
            name,
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A stand-in CLI. It records its arguments next to itself so tests can
    /// check the exact vector Knitt passes.
    fn fake_lfs(dir: &Path) -> PathBuf {
        let script = dir.join("git-lfs");
        std::fs::write(
            &script,
            r#"#!/bin/sh
printf '%s\n' "$@" > "$(dirname "$0")/args.txt"
case "$1" in
  version) echo "git-lfs/3.4.0 (test)" ;;
  track) printf 'Listing tracked patterns\n    *.psd (.gitattributes)\n    assets/** (.gitattributes)\nListing excluded patterns\n    *.zip (.gitattributes)\n' ;;
  *) exit 0 ;;
esac
"#,
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn repo(name: &str) -> PathBuf {
        let dir = temp_dir(name);
        Repository::init(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_the_tracked_block_only() {
        let output = "Listing tracked patterns\n    *.psd (.gitattributes)\n    assets/** (sub/.gitattributes)\nListing excluded patterns\n    *.zip (.gitattributes)\n";
        assert_eq!(
            parse_tracked(output),
            vec![
                TrackedPattern {
                    pattern: "*.psd".to_string(),
                    source: ".gitattributes".to_string(),
                },
                TrackedPattern {
                    pattern: "assets/**".to_string(),
                    source: "sub/.gitattributes".to_string(),
                },
            ]
        );
        assert!(parse_tracked("Listing tracked patterns\n").is_empty());
        assert!(parse_tracked("").is_empty());
    }

    #[test]
    fn patterns_are_validated_before_the_tool_runs() {
        assert_eq!(
            check_pattern("  ").unwrap_err(),
            "Enter a pattern such as *.psd."
        );
        assert_eq!(
            check_pattern("*.p sd").unwrap_err(),
            "A pattern cannot contain spaces."
        );
        assert_eq!(
            check_pattern("--all").unwrap_err(),
            "A pattern cannot start with a dash."
        );
        assert_eq!(check_pattern(" *.psd ").unwrap(), "*.psd");
    }

    #[test]
    fn status_reads_the_version_line() {
        let dir = temp_dir("status");
        let executable = fake_lfs(&dir);
        let status = status_of(&executable);
        assert!(status.installed);
        assert_eq!(status.version.as_deref(), Some("git-lfs/3.4.0 (test)"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_cli_reports_itself_as_not_installed() {
        let status = status_of(Path::new("/nonexistent/git-lfs"));
        assert!(!status.installed);
        assert_eq!(status.version, None);
    }

    #[test]
    fn listing_tracking_and_untracking_pass_the_pattern() {
        let dir = temp_dir("track");
        let executable = fake_lfs(&dir);
        let repo = repo("track-repo");
        let path = repo.to_str().unwrap();

        let patterns = tracked_at(&executable, path).unwrap();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].pattern, "*.psd");
        assert_eq!(std::fs::read_to_string(dir.join("args.txt")).unwrap(), "track\n");

        track_at(&executable, path, "*.bin").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("args.txt")).unwrap(),
            "track\n*.bin\n"
        );

        untrack_at(&executable, path, "*.bin").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("args.txt")).unwrap(),
            "untrack\n*.bin\n"
        );

        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn object_transfers_use_the_upstream_remote() {
        let dir = temp_dir("transfer");
        let executable = fake_lfs(&dir);
        let repo_dir = repo("transfer-repo");
        let repo = Repository::open(&repo_dir).unwrap();
        repo.remote("origin", "https://example.com/repo.git").unwrap();
        repo.remote("fork", "https://example.com/fork.git").unwrap();
        let path = repo_dir.to_str().unwrap();

        pull_at(&executable, path).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("args.txt")).unwrap(), "pull\n");

        push_at(&executable, path).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("args.txt")).unwrap(),
            "push\n--all\norigin\n"
        );

        let _ = std::fs::remove_dir_all(&repo_dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failing_tool_is_reported_without_its_output() {
        let dir = temp_dir("failure");
        let script = dir.join("git-lfs");
        std::fs::write(&script, "#!/bin/sh\necho 'secret detail' >&2\nexit 1\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let repo_dir = repo("failure-repo");

        let error = pull_at(&script, repo_dir.to_str().unwrap()).unwrap_err();
        assert_eq!(error, "Could not download the LFS objects.");
        assert!(!error.contains("secret"));

        let _ = std::fs::remove_dir_all(&repo_dir);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
