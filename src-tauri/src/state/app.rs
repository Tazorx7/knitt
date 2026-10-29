use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct DeviceFlowState {
    pub host: String,
    pub device_code: String,
    pub interval: u64,
    pub expires_at: i64,
    pub client_id: String,
}

#[derive(Default)]
pub struct AppState {
    pub device_flow: Arc<Mutex<Option<DeviceFlowState>>>,
    pub cancel: Arc<Mutex<bool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
