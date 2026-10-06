use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub security: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiStatus {
    pub enabled: bool,
    pub connected: bool,
    pub ssid: Option<String>,
    pub signal: Option<u8>,
}