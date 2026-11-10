use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// A log file larger than this is rotated to `knitt.log.1` at startup, so
/// retained logs stay bounded at roughly twice this size.
const MAX_LOG_BYTES: u64 = 1_048_576;

/// Directory holding Knitt's own logs. Kept under the app state directory so
/// clearing logs never touches settings or accounts.
pub fn log_dir() -> io::Result<PathBuf> {
    let dir = crate::config::paths::state_dir()?.join("logs");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn log_path() -> io::Result<PathBuf> {
    Ok(log_dir()?.join("knitt.log"))
}

/// Appends tracing output to the log file, opening it per write so rotation
/// and log clearing cannot leave a stale handle pointing at a deleted file.
struct LogFile(PathBuf);

impl Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.0)?;
        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> fmt::MakeWriter<'a> for LogFile {
    type Writer = LogFile;

    fn make_writer(&'a self) -> Self::Writer {
        LogFile(self.0.clone())
    }
}

fn rotate_if_needed(path: &PathBuf) {
    let too_large = fs::metadata(path)
        .map(|meta| meta.len() > MAX_LOG_BYTES)
        .unwrap_or(false);
    if too_large {
        let _ = fs::rename(path, path.with_file_name("knitt.log.1"));
    }
}

pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = fmt::layer().with_target(false);

    // Logging to a file must never stop the app from starting: if the log
    // directory cannot be created, keep stderr logging only.
    let file_layer = log_path().ok().map(|path| {
        rotate_if_needed(&path);
        fmt::layer()
            .with_target(false)
            .with_ansi(false)
            .with_writer(LogFile(path))
    });

    let base = tracing_subscriber::registry()
        .with(stderr_layer)
        .with(filter);

    match file_layer {
        Some(layer) => base.with(layer).init(),
        None => base.init(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_renames_oversized_log() {
        let dir = std::env::temp_dir().join(format!("knitt-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("knitt.log");

        fs::write(&path, vec![b'x'; (MAX_LOG_BYTES + 1) as usize]).unwrap();
        rotate_if_needed(&path);
        assert!(!path.exists());
        assert!(dir.join("knitt.log.1").exists());

        // A small file is left alone.
        fs::write(&path, b"small").unwrap();
        rotate_if_needed(&path);
        assert!(path.exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
