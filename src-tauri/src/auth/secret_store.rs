use keyring::Entry;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

pub struct SecretStore {
    service: String,
}

impl SecretStore {
    pub fn new(app_name: &str) -> Self {
        Self {
            service: format!("org.{}.github-auth", app_name),
        }
    }

    pub fn is_available(&self) -> bool {
        Entry::new(&self.service, "__probe__").is_ok()
    }

    pub fn set_token(&self, account_id: &str, token: &str) -> Result<(), SecretStoreError> {
        let entry = Entry::new(&self.service, account_id)
            .map_err(|e| SecretStoreError::Keyring(e.to_string()))?;
        entry
            .set_password(token)
            .map_err(|e| SecretStoreError::Keyring(e.to_string()))?;
        Ok(())
    }

    pub fn get_token(&self, account_id: &str) -> Result<Option<String>, SecretStoreError> {
        let entry = Entry::new(&self.service, account_id)
            .map_err(|e| SecretStoreError::Keyring(e.to_string()))?;
        match entry.get_password() {
            Ok(pw) => Ok(Some(pw)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretStoreError::Keyring(e.to_string())),
        }
    }

    pub fn delete_token(&self, account_id: &str) -> Result<(), SecretStoreError> {
        let entry = Entry::new(&self.service, account_id)
            .map_err(|e| SecretStoreError::Keyring(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretStoreError::Keyring(e.to_string())),
        }
    }
}
