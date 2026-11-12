//! Commit signing through the user's own `gpg` or `ssh-keygen`.
//!
//! Knitt builds the unsigned commit buffer with libgit2, asks the tool to sign
//! that exact byte sequence, and hands the signature back to libgit2. Private
//! key material is never read, copied or logged; only the configured key
//! identifier or file path is passed to the tool.

use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningMode {
    /// Write commits unsigned.
    Off,
    /// Use `gpg --detach-sign` with the configured key ID or email.
    Gpg,
    /// Use `ssh-keygen -Y sign` with the configured private key path.
    Ssh,
}

impl SigningMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "gpg" => Some(Self::Gpg),
            "ssh" => Some(Self::Ssh),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningConfig {
    pub mode: SigningMode,
    /// GPG key ID/email, or the path to an SSH private key.
    pub key: String,
}

impl SigningConfig {
    /// Returns `None` when signing is off, so callers can take the unsigned
    /// path without branching further.
    pub fn from_settings(mode: &str, key: &str) -> Option<Self> {
        let mode = SigningMode::parse(mode)?;
        if mode == SigningMode::Off {
            return None;
        }
        Some(Self {
            mode,
            key: key.trim().to_string(),
        })
    }
}

fn run_with_input(mut command: Command, input: &str) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "The signing tool is not available.".to_string())?;

    child
        .stdin
        .take()
        .ok_or_else(|| "The signing tool did not accept the commit.".to_string())?
        .write_all(input.as_bytes())
        .map_err(|_| "The signing tool did not accept the commit.".to_string())?;

    let output = child
        .wait_with_output()
        .map_err(|_| "The signing tool did not finish.".to_string())?;
    if !output.status.success() {
        return Err("Signing failed. Check the selected key and its passphrase.".to_string());
    }
    let signature = String::from_utf8(output.stdout)
        .map_err(|_| "The signing tool returned an unreadable signature.".to_string())?;
    if signature.trim().is_empty() {
        return Err("The signing tool returned an empty signature.".to_string());
    }
    Ok(signature)
}

fn sign_with_gpg(key: &str, content: &str) -> Result<String, String> {
    let mut command = Command::new("gpg");
    command
        .arg("--batch")
        .arg("--yes")
        .arg("--armor")
        .arg("--detach-sign");
    // An empty key means "use the default secret key".
    if !key.is_empty() {
        command.arg("--local-user").arg(key);
    }
    run_with_input(command, content)
}

fn sign_with_ssh(key: &str, content: &str) -> Result<String, String> {
    if key.is_empty() {
        return Err("Choose an SSH private key file before signing commits.".to_string());
    }
    let mut command = Command::new("ssh-keygen");
    command.arg("-Y").arg("sign").arg("-n").arg("git").arg("-f").arg(key);
    run_with_input(command, content)
}

/// Signs an unsigned commit buffer, returning the detached signature.
pub fn sign(content: &str, config: &SigningConfig) -> Result<String, String> {
    match config.mode {
        SigningMode::Off => Err("Signing is turned off.".to_string()),
        SigningMode::Gpg => sign_with_gpg(&config.key, content),
        SigningMode::Ssh => sign_with_ssh(&config.key, content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-sign-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn ssh_keygen_available() -> bool {
        Command::new("ssh-keygen")
            .arg("-?")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success() || status.code().is_some())
            .unwrap_or(false)
    }

    #[test]
    fn parses_only_known_modes() {
        assert_eq!(SigningMode::parse("off"), Some(SigningMode::Off));
        assert_eq!(SigningMode::parse("GPG"), Some(SigningMode::Gpg));
        assert_eq!(SigningMode::parse("ssh"), Some(SigningMode::Ssh));
        assert_eq!(SigningMode::parse("configured"), None);
    }

    #[test]
    fn off_produces_no_config() {
        assert!(SigningConfig::from_settings("off", "anything").is_none());
        assert!(SigningConfig::from_settings("unknown", "").is_none());
        assert_eq!(
            SigningConfig::from_settings("gpg", " user@example.com "),
            Some(SigningConfig {
                mode: SigningMode::Gpg,
                key: "user@example.com".to_string(),
            })
        );
    }

    #[test]
    fn ssh_requires_a_key_path() {
        let config = SigningConfig {
            mode: SigningMode::Ssh,
            key: String::new(),
        };
        assert_eq!(
            sign("commit", &config).unwrap_err(),
            "Choose an SSH private key file before signing commits."
        );
    }

    #[test]
    fn signs_with_ssh_when_available() {
        if !ssh_keygen_available() {
            return;
        }
        let dir = scratch("ssh");
        let key = dir.join("id_ed25519");
        let generated = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .status()
            .unwrap();
        assert!(generated.success());

        let config = SigningConfig {
            mode: SigningMode::Ssh,
            key: key.display().to_string(),
        };
        let signature = sign("unsigned commit contents\n", &config).unwrap();
        assert!(signature.starts_with("-----BEGIN SSH SIGNATURE-----"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_tool_is_reported_without_details() {
        let config = SigningConfig {
            mode: SigningMode::Ssh,
            key: "/nonexistent/key/file".to_string(),
        };
        let error = sign("commit", &config).unwrap_err();
        assert!(!error.contains("/nonexistent"));
    }
}
