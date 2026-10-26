use aether_services::{ConnectedWifiService, WifiScanner, WifiToggle};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub security: String,
    pub connected: bool,
}

/// Returns whether the Wi-Fi software radio is enabled.
#[tauri::command]
pub async fn wifi_is_enabled() -> Result<bool, String> {
    let wifi = WifiToggle::new().await.map_err(|e| e.to_string())?;

    wifi.is_enabled().await.map_err(|e| e.to_string())
}

/// Enables or disables the global Wi-Fi radio.
/// Returns the actual state reported by NetworkManager.
#[tauri::command]
pub async fn wifi_set_enabled(enabled: bool) -> Result<bool, String> {
    let wifi = WifiToggle::new().await.map_err(|e| e.to_string())?;

    if enabled {
        wifi.enable().await.map_err(|e| e.to_string())?;
    } else {
        wifi.disable().await.map_err(|e| e.to_string())?;
    }

    wifi.is_enabled().await.map_err(|e| e.to_string())
}

/// Scans for nearby Wi-Fi networks and returns them to the frontend.
#[tauri::command]
pub async fn wifi_scan_networks() -> Result<Vec<WifiNetwork>, String> {
    let toggle = WifiToggle::new().await.map_err(|e| e.to_string())?;

    // Do not scan when the Wi-Fi radio is disabled.
    let enabled = toggle.is_enabled().await.map_err(|e| e.to_string())?;

    if !enabled {
        return Ok(Vec::new());
    }

    // Use Aether's existing NetworkManager scanner.
    let scanner = WifiScanner::new().await.map_err(|e| e.to_string())?;

    let found = scanner.scan_and_list().await.map_err(|e| e.to_string())?;

    // Identify the currently connected network.
    let connected_ssid = match ConnectedWifiService::new().await {
        Ok(service) => match service.get().await {
            Ok(Some(details)) => Some(details.network.ssid),
            Ok(None) | Err(_) => None,
        },
        Err(_) => None,
    };

    // Convert Aether's scanner results into the frontend's data format.
    let mut networks: Vec<WifiNetwork> = found
        .into_iter()
        .filter(|network| !network.ssid.is_empty())
        .map(|network| {
            let connected = connected_ssid.as_deref() == Some(network.ssid.as_str());

            WifiNetwork {
                ssid: network.ssid,
                signal: network.strength.unwrap_or(0),
                security: if network.secured {
                    "Secured".to_string()
                } else {
                    "Open".to_string()
                },
                connected,
            }
        })
        .collect();

    // Display the strongest networks first.
    networks.sort_by(|a, b| b.signal.cmp(&a.signal));

    // Remove duplicate SSIDs, keeping the strongest entry.
    networks.dedup_by(|a, b| a.ssid == b.ssid);

    Ok(networks)
}
