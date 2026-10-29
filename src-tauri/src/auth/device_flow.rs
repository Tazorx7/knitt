use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct DeviceCodeRequest {
    pub client_id: String,
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
pub struct TokenRequest {
    pub client_id: String,
    pub device_code: String,
    pub grant_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenResponse {
    Success {
        access_token: String,
        token_type: String,
        scope: String,
    },
    Error {
        error: String,
        error_description: Option<String>,
    },
}

pub fn device_code_url(host: &str) -> String {
    if host == "github.com" {
        "https://github.com/login/device/code".to_string()
    } else {
        format!("https://{}/login/device/code", host)
    }
}

pub fn token_url(host: &str) -> String {
    if host == "github.com" {
        "https://github.com/login/oauth/access_token".to_string()
    } else {
        format!("https://{}/login/oauth/access_token", host)
    }
}

pub fn api_base(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com".to_string()
    } else {
        format!("https://{}/api/v3", host)
    }
}
