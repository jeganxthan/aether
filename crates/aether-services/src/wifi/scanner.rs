use nmrs::{Network, NetworkManager};
use tokio::time::{Duration, sleep};

pub struct WifiScanner {
    manager: NetworkManager,
}

impl WifiScanner {
    pub async fn new() -> nmrs::Result<Self> {
        let manager = NetworkManager::new().await?;

        Ok(Self { manager })
    }

    /// Request a fresh scan of all Wi-Fi devices.
    pub async fn scan(&self) -> nmrs::Result<()> {
        self.manager.scan_networks(None).await
    }

    /// Return networks currently known to NetworkManager.
    pub async fn available_networks(&self) -> nmrs::Result<Vec<Network>> {
        self.manager.list_networks(None).await
    }

    /// Scan, wait briefly, then return the discovered networks.
    pub async fn scan_and_list(&self) -> nmrs::Result<Vec<Network>> {
        self.scan().await?;

        // Scanning happens asynchronously in NetworkManager.
        sleep(Duration::from_secs(3)).await;

        let mut networks = self.available_networks().await?;

        // Strongest signal first.
        networks.sort_by(|a, b| b.strength.unwrap_or(0).cmp(&a.strength.unwrap_or(0)));

        Ok(networks)
    }
}
