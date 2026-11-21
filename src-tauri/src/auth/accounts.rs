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

/// Retrieve the active account's OAuth token from the secret store.
/// Returns `None` when there is no active account or no stored token.
pub fn active_token() -> Result<Option<String>, String> {
    let store = AccountsStore::load().map_err(|error| {
        tracing::warn!(%error, "could not read the saved accounts");
        "Could not read the saved GitHub accounts.".to_string()
    })?;
    let account_id = match &store.active_account_id {
        Some(id) => id.clone(),
        None => return Ok(None),
    };
    let secret = crate::auth::secret_store::SecretStore::new("nativegitclient");
    secret.get_token(&account_id).map_err(|error| {
        tracing::warn!(%error, "could not read the stored token");
        "Could not read the saved GitHub token. Sign in again.".to_string()
    })
}

/// Retrieve the token for a repository on `host`, so an Enterprise host uses
/// its own account instead of whichever account happens to be active.
pub fn token_for_host(host: &str) -> Result<Option<String>, String> {
    let store = AccountsStore::load().map_err(|error| {
        tracing::warn!(%error, "could not read the saved accounts");
        "Could not read the saved GitHub accounts.".to_string()
    })?;
    let Some(account) = store.account_for_host(host) else {
        return Ok(None);
    };
    let secret = crate::auth::secret_store::SecretStore::new("nativegitclient");
    secret.get_token(&account.id).map_err(|error| {
        tracing::warn!(%error, "could not read the stored token");
        "Could not read the saved GitHub token. Sign in again.".to_string()
    })
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

    /// The account that should be used for a repository hosted on `host`: the
    /// active account when it belongs to that host, otherwise the account
    /// marked default for the host, otherwise the first account that matches.
    pub fn account_for_host(&self, host: &str) -> Option<&GitHubAccount> {
        if let Some(active) = self.active_account_id.as_deref()
            && let Some(account) = self
                .accounts
                .iter()
                .find(|account| account.id == active && account.host == host)
        {
            return Some(account);
        }
        self.accounts
            .iter()
            .find(|account| account.host == host && account.is_default_for_host)
            .or_else(|| self.accounts.iter().find(|account| account.host == host))
    }

    pub fn remove(&mut self, id: &str) {
        self.accounts.retain(|a| a.id != id);
        if self.active_account_id.as_deref() == Some(id) {
            self.active_account_id = self.accounts.first().map(|a| a.id.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str, host: &str, default_for_host: bool) -> GitHubAccount {
        GitHubAccount {
            id: id.to_string(),
            host: host.to_string(),
            login: id.to_string(),
            name: None,
            avatar_url: String::new(),
            scopes: Vec::new(),
            added_at: 0,
            is_default_for_host: default_for_host,
        }
    }

    #[test]
    fn picks_the_active_account_when_it_belongs_to_the_host() {
        let store = AccountsStore {
            accounts: vec![
                account("a", "github.com", false),
                account("b", "ghe.example.com", false),
            ],
            active_account_id: Some("b".to_string()),
        };
        assert_eq!(
            store.account_for_host("ghe.example.com").map(|a| a.id.as_str()),
            Some("b")
        );
    }

    #[test]
    fn falls_back_to_an_account_for_the_same_host() {
        let store = AccountsStore {
            accounts: vec![
                account("a", "github.com", false),
                account("b", "ghe.example.com", false),
            ],
            active_account_id: Some("a".to_string()),
        };
        assert_eq!(
            store.account_for_host("ghe.example.com").map(|a| a.id.as_str()),
            Some("b"),
            "another host's account is still usable"
        );
        assert!(store.account_for_host("gitlab.example.com").is_none());
    }

    #[test]
    fn prefers_the_account_marked_default_for_the_host() {
        let store = AccountsStore {
            accounts: vec![
                account("a", "github.com", false),
                account("b", "ghe.example.com", false),
                account("c", "ghe.example.com", true),
            ],
            active_account_id: Some("a".to_string()),
        };
        assert_eq!(
            store.account_for_host("ghe.example.com").map(|a| a.id.as_str()),
            Some("c"),
            "the host's own default beats an unrelated active account"
        );
    }

    #[test]
    fn no_accounts_means_no_account_for_a_host() {
        let store = AccountsStore::default();
        assert!(store.account_for_host("github.com").is_none());
    }
}
