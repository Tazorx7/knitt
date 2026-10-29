use crate::auth::{
    accounts::{AccountsStore, GitHubAccount},
    device_flow::{self, DeviceCodeResponse, TokenResponse},
    secret_store::SecretStore,
};
use crate::state::app::{AppState, DeviceFlowState};
use chrono::Utc;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Serialize;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

const CLIENT_ID: &str = "Ov23liPlaceholderClientId"; // TODO: replace with real OAuth app client_id

#[derive(Serialize)]
pub struct DeviceCodeView {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[tauri::command]
pub async fn auth_check_secret_service() -> Result<bool, String> {
    let store = SecretStore::new("nativegitclient");
    Ok(store.is_available())
}

#[tauri::command]
pub async fn auth_start_device_flow(app: AppHandle, state: State<'_, AppState>, host: Option<String>) -> Result<DeviceCodeView, String> {
    let host = host.unwrap_or_else(|| "github.com".to_string());
    let client = reqwest::Client::new();
    let url = device_flow::device_code_url(&host);
    let mut body = HashMap::new();
    body.insert("client_id", CLIENT_ID);
    body.insert("scope", "repo read:org workflow read:user");
    
    let resp = client
        .post(&url)
        .header(USER_AGENT, "nativegitclient/0.1.0")
        .header(ACCEPT, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    let code: DeviceCodeResponse = resp.json().await.map_err(|e| e.to_string())?;
    
    let mut flow = state.device_flow.lock().await;
    *flow = Some(DeviceFlowState {
        host: host.clone(),
        device_code: code.device_code.clone(),
        interval: if code.interval == 0 { 5 } else { code.interval },
        expires_at: Utc::now().timestamp() + code.expires_in as i64,
        client_id: CLIENT_ID.to_string(),
    });
    *state.cancel.lock().await = false;
    
    let _ = app.emit("auth:code", serde_json::json!({
        "host": host,
        "user_code": code.user_code,
        "verification_uri": code.verification_uri,
        "expires_in": code.expires_in,
    }));
    
    Ok(DeviceCodeView {
        user_code: code.user_code,
        verification_uri: code.verification_uri,
        expires_in: code.expires_in,
    })
}

#[tauri::command]
pub async fn auth_cancel_device_flow(state: State<'_, AppState>) -> Result<(), String> {
    let mut flow = state.device_flow.lock().await;
    *flow = None;
    *state.cancel.lock().await = true;
    Ok(())
}

#[tauri::command]
pub async fn auth_poll_device_flow(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let flow = {
        let f = state.device_flow.lock().await;
        match f.clone() {
            Some(x) => x,
            None => return Ok(false),
        }
    };
    
    if Utc::now().timestamp() > flow.expires_at {
        let _ = app.emit("auth:expired", serde_json::json!({}));
        let mut f2 = state.device_flow.lock().await;
        *f2 = None;
        return Ok(false);
    }
    
    let client = reqwest::Client::new();
    let url = device_flow::token_url(&flow.host);
    let mut body = HashMap::new();
    body.insert("client_id", flow.client_id.as_str());
    body.insert("device_code", flow.device_code.as_str());
    body.insert("grant_type", "urn:ietf:params:oauth:grant-type:device_code");
    
    tokio::time::sleep(tokio::time::Duration::from_secs(flow.interval)).await;
    
    if *state.cancel.lock().await {
        return Ok(false);
    }
    
    let resp = client
        .post(&url)
        .header(USER_AGENT, "nativegitclient/0.1.0")
        .header(ACCEPT, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let token_resp: TokenResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    
    match token_resp {
        TokenResponse::Success { access_token, scope, .. } => {
            let api = device_flow::api_base(&flow.host);
            let user_resp = client
                .get(format!("{}/user", api))
                .header(USER_AGENT, "nativegitclient/0.1.0")
                .header(ACCEPT, "application/vnd.github+json")
                .bearer_auth(&access_token)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let user: serde_json::Value = user_resp.json().await.map_err(|e| e.to_string())?;
            
            let login = user["login"].as_str().unwrap_or("").to_string();
            let name = user["name"].as_str().map(|s| s.to_string());
            let avatar_url = user["avatar_url"].as_str().unwrap_or("").to_string();
            let id = format!("{}:{}", flow.host, login);
            
            let account = GitHubAccount {
                id: id.clone(),
                host: flow.host.clone(),
                login: login.clone(),
                name,
                avatar_url,
                scopes: scope.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                added_at: Utc::now().timestamp(),
                is_default_for_host: false,
            };
            
            let store = SecretStore::new("nativegitclient");
            store.set_token(&id, &access_token).map_err(|e| e.to_string())?;
            
            let mut accounts = AccountsStore::load().unwrap_or_default();
            accounts.upsert(account.clone());
            let _ = accounts.save();
            
            let mut f2 = state.device_flow.lock().await;
            *f2 = None;
            
            let _ = app.emit("auth:success", serde_json::json!({
                "account": account
            }));
            Ok(true)
        }
        TokenResponse::Error { error, .. } => {
            if error == "authorization_pending" {
                let _ = app.emit("auth:pending", serde_json::json!({}));
                Ok(false)
            } else if error == "slow_down" {
                let mut f2 = state.device_flow.lock().await;
                if let Some(ref mut f) = *f2 {
                    f.interval += 5;
                }
                let _ = app.emit("auth:pending", serde_json::json!({"slow_down": true}));
                Ok(false)
            } else if error == "expired_token" {
                let _ = app.emit("auth:expired", serde_json::json!({}));
                let mut f2 = state.device_flow.lock().await;
                *f2 = None;
                Ok(false)
            } else if error == "access_denied" {
                let _ = app.emit("auth:error", serde_json::json!({"error": "access_denied"}));
                let mut f2 = state.device_flow.lock().await;
                *f2 = None;
                Ok(false)
            } else {
                let _ = app.emit("auth:error", serde_json::json!({"error": error}));
                Ok(false)
            }
        }
    }
}

#[tauri::command]
pub async fn auth_list_accounts() -> Result<serde_json::Value, String> {
    let store = AccountsStore::load().unwrap_or_default();
    Ok(serde_json::json!({
        "accounts": store.accounts,
        "active_account_id": store.active_account_id
    }))
}

#[tauri::command]
pub async fn auth_sign_out(account_id: String) -> Result<(), String> {
    let secret = SecretStore::new("nativegitclient");
    let _ = secret.delete_token(&account_id);
    let mut store = AccountsStore::load().unwrap_or_default();
    store.remove(&account_id);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn auth_sign_out_all() -> Result<(), String> {
    let secret = SecretStore::new("nativegitclient");
    let mut store = AccountsStore::load().unwrap_or_default();
    for acc in &store.accounts {
        let _ = secret.delete_token(&acc.id);
    }
    store.accounts.clear();
    store.active_account_id = None;
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn auth_set_active_account(account_id: String) -> Result<(), String> {
    let mut store = AccountsStore::load().unwrap_or_default();
    if store.accounts.iter().any(|a| a.id == account_id) {
        store.active_account_id = Some(account_id);
        store.save().map_err(|e| e.to_string())?;
    }
    Ok(())
}
