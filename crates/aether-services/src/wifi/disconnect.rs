use nmrs::NetworkManager;

/// Disconnect from the current Wi-Fi network.
///
/// This does not disable the Wi-Fi radio or delete
/// the saved connection profile.
pub async fn disconnect_wifi(manager: &NetworkManager) -> nmrs::Result<()> {
    manager.disconnect(None).await
}

/// Disconnect from Wi-Fi on a specific interface,
/// for example "wlp2s0".
pub async fn disconnect_wifi_interface(
    manager: &NetworkManager,
    interface: &str,
) -> nmrs::Result<()> {
    manager.disconnect(Some(interface)).await
}
