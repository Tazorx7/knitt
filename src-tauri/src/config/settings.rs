use crate::config::paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            git_user_name: None,
            git_user_email: None,
        }
    }
}

impl Settings {
    fn path() -> std::io::Result<PathBuf> {
        Ok(paths::config_dir()?.join("settings.json"))
    }

    pub fn load() -> std::io::Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        let s: Settings = serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(s)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, data)?;
        Ok(())
    }
}
