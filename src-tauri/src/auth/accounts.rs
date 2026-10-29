use crate::config::paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAccount {
    pub id: String,
    pub host: String,
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: String,
    pub scopes: Vec<String>,
    pub added_at: i64,
    pub is_default_for_host: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountsStore {
    pub accounts: Vec<GitHubAccount>,
    pub active_account_id: Option<String>,
}

impl AccountsStore {
    fn accounts_path() -> std::io::Result<PathBuf> {
        Ok(paths::data_dir()?.join("accounts.json"))
    }

    pub fn load() -> std::io::Result<Self> {
        let path = Self::accounts_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        let store: AccountsStore = serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(store)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::accounts_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn upsert(&mut self, account: GitHubAccount) {
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.id == account.id) {
            *existing = account.clone();
        } else {
            self.accounts.push(account.clone());
        }
        if self.active_account_id.is_none() {
            self.active_account_id = Some(account.id);
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.accounts.retain(|a| a.id != id);
        if self.active_account_id.as_deref() == Some(id) {
            self.active_account_id = self.accounts.first().map(|a| a.id.clone());
        }
    }
}
