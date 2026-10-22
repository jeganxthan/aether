use nmrs::NetworkManager;

pub struct WifiToggle {
    manager: NetworkManager,
}

impl WifiToggle {
    pub async fn new() -> nmrs::Result<Self> {
        let manager = NetworkManager::new().await?;
        Ok(Self { manager })
    }

    /// Returns whether the Wi-Fi software radio is enabled.
    pub async fn is_enabled(&self) -> nmrs::Result<bool> {
        let state = self.manager.wifi_state().await?;
        Ok(state.enabled)
    }

    /// Enable the global Wi-Fi radio.
    pub async fn enable(&self) -> nmrs::Result<()> {
        self.manager.set_wireless_enabled(true).await
    }

    /// Disable the global Wi-Fi radio.
    pub async fn disable(&self) -> nmrs::Result<()> {
        self.manager.set_wireless_enabled(false).await
    }

    /// Switch Wi-Fi to the opposite of its current state.
    pub async fn toggle(&self) -> nmrs::Result<bool> {
        let enabled = self.is_enabled().await?;
        let next_state = !enabled;

        self.manager.set_wireless_enabled(next_state).await?;

        Ok(next_state)
    }
}
