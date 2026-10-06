use crate::{
    models::{WifiNetwork, WifiStatus},
    services::WifiService,
};

#[tauri::command]
pub fn wifi_status() -> Result<WifiStatus, String> {
    WifiService::new()
        .status()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    WifiService::new()
        .networks()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn wifi_enable() -> Result<(), String> {
    WifiService::new()
        .enable()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn wifi_disable() -> Result<(), String> {
    WifiService::new()
        .disable()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn wifi_rescan() -> Result<(), String> {
    WifiService::new()
        .rescan()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn wifi_connect(
    ssid: String,
    password: Option<String>,
) -> Result<(), String> {
    WifiService::new()
        .connect(
            &ssid,
            password.as_deref(),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn wifi_disconnect() -> Result<(), String> {
    WifiService::new()
        .disconnect()
        .map_err(|error| error.to_string())
}