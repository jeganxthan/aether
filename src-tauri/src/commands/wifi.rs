use crate::{
    models::{WifiNetwork, WifiStatus},
    services::WifiService,
};

/*
 * Every WifiService call shells out to nmcli, and several of them wait on
 * NetworkManager (rescan, connect) for multiple seconds. As plain sync
 * commands they run inline on the GTK main thread, so the whole island
 * froze until nmcli returned. Run the blocking work off the UI thread.
 */
async fn off_ui_thread<T, F>(work: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result)
}

#[tauri::command]
pub async fn wifi_status() -> Result<WifiStatus, String> {
    off_ui_thread(|| {
        WifiService::new()
            .status()
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
pub async fn wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    off_ui_thread(|| {
        WifiService::new()
            .networks()
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
pub async fn wifi_enable() -> Result<(), String> {
    off_ui_thread(|| {
        WifiService::new()
            .enable()
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
pub async fn wifi_disable() -> Result<(), String> {
    off_ui_thread(|| {
        WifiService::new()
            .disable()
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
pub async fn wifi_rescan() -> Result<(), String> {
    off_ui_thread(|| {
        WifiService::new()
            .rescan()
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
pub async fn wifi_connect(
    ssid: String,
    password: Option<String>,
) -> Result<(), String> {
    off_ui_thread(move || {
        WifiService::new()
            .connect(&ssid, password.as_deref())
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
pub async fn wifi_disconnect() -> Result<(), String> {
    off_ui_thread(|| {
        WifiService::new()
            .disconnect()
            .map_err(|error| error.to_string())
    })
    .await
}
