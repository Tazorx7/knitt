use std::path::PathBuf;

/// Directory under every XDG base. Deliberately short: it is the folder users
/// see, and it is not the application identifier.
pub const APP_DIR: &str = "knitt";

pub fn config_dir() -> std::io::Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "config dir not found"))?
        .join(APP_DIR);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn data_dir() -> std::io::Result<PathBuf> {
    let dir = dirs::data_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "data dir not found"))?
        .join(APP_DIR);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn state_dir() -> std::io::Result<PathBuf> {
    let dir = dirs::state_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "state dir not found"))?
        .join(APP_DIR);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
