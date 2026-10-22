use nmrs::{NetworkManager, SettingsSummary, WifiSecurity};

/// Connect to a saved Wi-Fi network.
///
/// If the network has a saved profile, NetworkManager can reuse
/// its stored credentials.
pub async fn connect_saved(manager: &NetworkManager, ssid: &str) -> nmrs::Result<()> {
    let profiles = manager.list_saved_connections().await?;
    if let Some(profile) = profiles
        .into_iter()
        .find(|profile| matches!(&profile.summary, SettingsSummary::Wifi { ssid: saved_ssid, .. } if saved_ssid == ssid))
    {
        return manager
            .connect_by_uuid(&profile.uuid, nmrs::ConnectByUuidConfig::default())
            .await;
    }

    manager.connect(ssid, None, WifiSecurity::Open).await
}

/// Connect to a new or existing WPA-Personal network using a password.
pub async fn connect_with_password(
    manager: &NetworkManager,
    ssid: &str,
    password: &str,
) -> nmrs::Result<()> {
    manager
        .connect(
            ssid,
            None,
            WifiSecurity::WpaPsk {
                psk: password.to_owned(),
            },
        )
        .await
}

/// Connect to an open Wi-Fi network.
pub async fn connect_open(manager: &NetworkManager, ssid: &str) -> nmrs::Result<()> {
    manager.connect(ssid, None, WifiSecurity::Open).await
}
