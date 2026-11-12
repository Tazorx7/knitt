//! Read-only SSH environment diagnostics.
//!
//! Knitt authenticates SSH remotes through the running SSH agent (libgit2's
//! `ssh_key_from_agent`). Nothing here reads private key material, uploads
//! keys, edits `~/.ssh/config` or launches the `ssh` binary.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Default)]
pub struct SshDiagnostics {
    /// An `ssh` executable is present on PATH. Informational only: Knitt does
    /// not run it.
    pub ssh_executable: bool,
    /// `SSH_AUTH_SOCK` points at a real socket.
    pub agent_available: bool,
    /// Public key file names in `~/.ssh`, never their contents.
    pub public_keys: Vec<String>,
    /// Whether a repository was inspected.
    pub repository_checked: bool,
    /// How many of the repository's remotes use an SSH URL.
    pub ssh_remotes: usize,
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.is_file()
            && std::fs::metadata(path)
                .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn executable_on_path(binary: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| is_executable(&dir.join(binary)))
        })
        .unwrap_or(false)
}

/// Checks that a path is a Unix socket without connecting to it.
fn is_socket(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.file_type().is_socket())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn is_ssh_url(url: &str) -> bool {
    let url = url.trim();
    url.starts_with("ssh://") || (url.starts_with("git@") && url.contains(':'))
}

fn ssh_remote_count(path: &str) -> usize {
    match git2::Repository::open(path) {
        Ok(repo) => match repo.remotes() {
            Ok(names) => names
                .iter()
                .flatten()
                .filter_map(|name| repo.find_remote(name).ok())
                .filter(|remote| remote.url().is_some_and(is_ssh_url))
                .count(),
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Public key file names in a directory. Contents are never read.
fn public_keys_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .filter(|name| name.ends_with(".pub"))
        .collect();
    keys.sort();
    keys
}

fn ssh_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".ssh"))
}

/// Collects the diagnostics. `repo_path` is optional so the section can render
/// without an open repository.
pub fn diagnose(repo_path: Option<&str>) -> SshDiagnostics {
    let agent_available = std::env::var_os("SSH_AUTH_SOCK")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .is_some_and(|path| is_socket(&path));

    let public_keys = ssh_dir().map(|dir| public_keys_in(&dir)).unwrap_or_default();
    let ssh_remotes = repo_path.map(ssh_remote_count).unwrap_or(0);

    SshDiagnostics {
        ssh_executable: executable_on_path("ssh"),
        agent_available,
        public_keys,
        repository_checked: repo_path.is_some(),
        ssh_remotes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-ssh-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lists_public_keys_by_name_only() {
        let dir = scratch("keys");
        std::fs::write(dir.join("id_ed25519"), "PRIVATE").unwrap();
        std::fs::write(dir.join("id_ed25519.pub"), "ssh-ed25519 AAAA").unwrap();
        std::fs::write(dir.join("work.pub"), "ssh-rsa BBBB").unwrap();
        std::fs::create_dir(dir.join("nested.pub")).unwrap();

        assert_eq!(
            public_keys_in(&dir),
            vec!["id_ed25519.pub".to_string(), "work.pub".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_ssh_urls() {
        assert!(is_ssh_url("git@github.com:owner/repo.git"));
        assert!(is_ssh_url("ssh://git@github.com/owner/repo.git"));
        assert!(!is_ssh_url("https://github.com/owner/repo.git"));
        assert!(!is_ssh_url("git@github.com"));
    }

    #[cfg(unix)]
    #[test]
    fn socket_detection_rejects_regular_files() {
        let dir = scratch("socket");
        std::fs::write(dir.join("plain"), "").unwrap();
        assert!(!is_socket(&dir.join("plain")));

        let socket = dir.join("agent.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        assert!(is_socket(&socket));
        drop(listener);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repository_without_remotes_reports_zero_ssh_remotes() {
        let dir = scratch("repo");
        git2::Repository::init(&dir).unwrap();
        let diagnostics = diagnose(Some(dir.to_str().unwrap()));
        assert!(diagnostics.repository_checked);
        assert_eq!(diagnostics.ssh_remotes, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_directory_lists_no_keys() {
        let dir = scratch("missing");
        let absent = dir.join("not-there");
        assert!(public_keys_in(&absent).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
