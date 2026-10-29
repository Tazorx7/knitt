use crate::auth::{accounts::AccountsStore, secret_store::SecretStore};
use git2::{Cred, RemoteCallbacks};

pub fn make_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, _allowed_types| {
        // For ssh
        if url.starts_with("git@") || url.starts_with("ssh://") {
            return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
        }
        // For https
        let host = if let Ok(parsed) = url::Url::parse(url) {
            parsed.host_str().unwrap_or("github.com").to_string()
        } else {
            "github.com".to_string()
        };
        let store = AccountsStore::load().map_err(|e| git2::Error::from_str(&e.to_string()))?;
        let secret = SecretStore::new("nativegitclient");
        for acc in &store.accounts {
            if acc.host == host
                && let Ok(Some(token)) = secret.get_token(&acc.id)
            {
                return Cred::userpass_plaintext("x-access-token", &token);
            }
        }
        Err(git2::Error::from_str("no credentials found"))
    });
    callbacks
}
