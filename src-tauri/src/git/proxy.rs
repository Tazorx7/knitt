use git2::ProxyOptions;

/// How network operations reach the internet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyMode {
    /// Use the proxy configured in Git or the environment.
    System,
    /// Never use a proxy.
    None,
    /// Use the URL the user entered.
    Manual,
}

impl ProxyMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "system" => Some(Self::System),
            "none" => Some(Self::None),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::None => "none",
            Self::Manual => "manual",
        }
    }
}

/// A validated proxy choice. `url` is only meaningful for `Manual`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub mode: ProxyMode,
    pub url: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            mode: ProxyMode::System,
            url: String::new(),
        }
    }
}

impl ProxyConfig {
    /// Builds a config from stored settings, ignoring unusable combinations the
    /// same way the settings normaliser does.
    pub fn from_settings(mode: &str, url: &str) -> Self {
        let mode = ProxyMode::parse(mode).unwrap_or(ProxyMode::System);
        let url = url.trim().to_string();
        // An empty manual address is allowed while the user is still typing;
        // it simply means "connect directly" until an address is entered.
        if mode == ProxyMode::Manual && !url.is_empty() && validate_proxy_url(&url).is_err() {
            return Self::default();
        }
        Self { mode, url }
    }

    /// libgit2 proxy options for this choice. Errors only if a manual URL has
    /// become invalid since it was stored.
    pub fn options(&self) -> Result<ProxyOptions<'static>, String> {
        let mut options = ProxyOptions::new();
        match self.mode {
            ProxyMode::System => {
                options.auto();
            }
            ProxyMode::None => {}
            ProxyMode::Manual => {
                if !self.url.is_empty() {
                    validate_proxy_url(&self.url)?;
                    options.url(&self.url);
                }
            }
        }
        Ok(options)
    }
}

/// Accepts `http`, `https`, `socks5` and `socks5h` proxy URLs without
/// credentials. Credentials are rejected rather than stored in plaintext.
pub fn validate_proxy_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("Enter a proxy address.".to_string());
    }

    let rest = ["http://", "https://", "socks5://", "socks5h://"]
        .iter()
        .find_map(|scheme| url.strip_prefix(scheme))
        .ok_or_else(|| "Use an http://, https:// or socks5:// proxy address.".to_string())?;

    // Everything before the first `/` is authority; a `@` there means the URL
    // embeds a username or password.
    let authority = rest.split('/').next().unwrap_or("");
    if authority.contains('@') {
        return Err("Proxy addresses must not include a username or password.".to_string());
    }
    let host = authority.rsplit_once(':').map_or(authority, |(host, _)| host);
    if host.is_empty() {
        return Err("Enter a proxy host.".to_string());
    }
    if url.chars().any(char::is_whitespace) {
        return Err("Proxy addresses cannot contain spaces.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_known_modes() {
        assert_eq!(ProxyMode::parse("system"), Some(ProxyMode::System));
        assert_eq!(ProxyMode::parse("NONE"), Some(ProxyMode::None));
        assert_eq!(ProxyMode::parse("manual"), Some(ProxyMode::Manual));
        assert_eq!(ProxyMode::parse("auto"), None);
    }

    #[test]
    fn accepts_plain_proxy_urls() {
        assert!(validate_proxy_url("http://proxy.example:3128").is_ok());
        assert!(validate_proxy_url("https://proxy.example").is_ok());
        assert!(validate_proxy_url("socks5://127.0.0.1:9050").is_ok());
        assert!(validate_proxy_url("socks5h://localhost:1080").is_ok());
    }

    #[test]
    fn rejects_unsupported_and_credentialled_urls() {
        assert!(validate_proxy_url("").is_err());
        assert!(validate_proxy_url("proxy.example:3128").is_err());
        assert!(validate_proxy_url("ftp://proxy.example").is_err());
        assert_eq!(
            validate_proxy_url("http://user:pass@proxy.example:3128").unwrap_err(),
            "Proxy addresses must not include a username or password."
        );
        assert!(validate_proxy_url("http://").is_err());
        assert!(validate_proxy_url("http://proxy example").is_err());
    }

    #[test]
    fn manual_configs_with_invalid_urls_fall_back_to_system() {
        let config = ProxyConfig::from_settings("manual", "http://user:pass@proxy:8080");
        assert_eq!(config, ProxyConfig::default());

        let config = ProxyConfig::from_settings("manual", " http://proxy.example:8080 ");
        assert_eq!(config.mode, ProxyMode::Manual);
        assert_eq!(config.url, "http://proxy.example:8080");

        let config = ProxyConfig::from_settings("manual", "");
        assert_eq!(config.mode, ProxyMode::Manual);
    }

    #[test]
    fn options_match_the_mode() {
        assert!(ProxyConfig::default().options().is_ok());
        assert!(ProxyConfig {
            mode: ProxyMode::None,
            url: String::new()
        }
        .options()
        .is_ok());
        assert!(ProxyConfig {
            mode: ProxyMode::Manual,
            url: "http://proxy.example:8080".to_string()
        }
        .options()
        .is_ok());
        assert!(ProxyConfig {
            mode: ProxyMode::Manual,
            url: String::new()
        }
        .options()
        .is_ok());
    }
}
