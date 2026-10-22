use nmrs::NetworkManager;

/// Joining and leaving the current Wi-Fi network.
///
/// Owns its own NetworkManager, so a caller only names the network it wants and
/// does not have to reach for the bus itself.
pub struct WifiConnection {
    manager: NetworkManager,
}

impl WifiConnection {
    pub async fn new() -> nmrs::Result<Self> {
        let manager = NetworkManager::new().await?;

        Ok(Self { manager })
    }

    /// Join a network with a password, for a secured one without a saved profile.
    pub async fn join_with_password(&self, ssid: &str, password: &str) -> nmrs::Result<()> {
        super::connect::connect_with_password(&self.manager, ssid, password).await
    }

    /// Join a network from its saved profile, or as an open one when there is none.
    pub async fn join_saved(&self, ssid: &str) -> nmrs::Result<()> {
        super::connect::connect_saved(&self.manager, ssid).await
    }

    /// Leave the current network, keeping the radio on and the profile saved.
    pub async fn leave(&self) -> nmrs::Result<()> {
        super::disconnect::disconnect_wifi(&self.manager).await
    }
}
