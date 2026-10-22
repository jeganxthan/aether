pub mod connect;
mod connected;
mod connection;
pub mod disconnect;
pub mod errors;
pub mod forget;
pub mod saved_networks;
pub mod saved_password;
mod scanner;
mod toggle;

use nmrs::{NetworkManager, SettingsSummary};

pub use connected::{ConnectedWifiDetails, ConnectedWifiService, ConnectedWifiSignal};
pub use connection::WifiConnection;
pub use scanner::WifiScanner;
pub use toggle::WifiToggle;

/// Return the SSIDs that have saved NetworkManager profiles.
pub async fn saved_wifi_ssids() -> nmrs::Result<Vec<String>> {
    let manager = NetworkManager::new().await?;
    Ok(manager
        .list_saved_connections()
        .await?
        .into_iter()
        .filter_map(|profile| match profile.summary {
            SettingsSummary::Wifi { ssid, .. } if !ssid.is_empty() => Some(ssid),
            _ => None,
        })
        .collect())
}

pub async fn forget_saved_wifi(ssid: &str) -> Result<(), forget::ForgetWifiError> {
    let manager = NetworkManager::new()
        .await
        .map_err(forget::ForgetWifiError::NetworkManager)?;
    forget::forget_saved_network(&manager, ssid).await
}

pub async fn ethernet_connected() -> nmrs::Result<bool> {
    let manager = NetworkManager::new().await?;
    Ok(manager
        .list_devices()
        .await?
        .iter()
        .any(|device| device.is_wired() && matches!(device.state, nmrs::DeviceState::Activated)))
}
