use nmrs::{NetworkInfo, NetworkManager, WifiDevice};

#[derive(Debug)]
pub struct ConnectedWifiDetails {
    pub network: NetworkInfo,
    pub device: Option<WifiDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedWifiSignal {
    pub strength: u8,
    pub secured: bool,
}

pub struct ConnectedWifiService {
    manager: NetworkManager,
}

impl ConnectedWifiService {
    pub async fn new() -> nmrs::Result<Self> {
        Ok(Self {
            manager: NetworkManager::new().await?,
        })
    }

    /// Read the active Wi-Fi device directly instead of resolving the current
    /// access point. The latter can briefly return `None` while its details are
    /// being refreshed even though NetworkManager still has an active device.
    pub async fn active_ssid(&self) -> nmrs::Result<Option<String>> {
        Ok(self
            .manager
            .list_wifi_devices()
            .await?
            .into_iter()
            .find(|device| device.is_active)
            .and_then(|device| device.active_ssid))
    }

    /// Get details for the currently connected Wi-Fi network.
    pub async fn get(&self) -> nmrs::Result<Option<ConnectedWifiDetails>> {
        let Some(network) = self.manager.current_network().await? else {
            return Ok(None);
        };

        let details = self.manager.show_details(&network).await?;
        let devices = self.manager.list_wifi_devices().await?;

        let device = devices.into_iter().find(|device| {
            device.is_active && device.active_ssid.as_deref() == Some(details.ssid.as_str())
        });

        Ok(Some(ConnectedWifiDetails {
            network: details,
            device,
        }))
    }

    /// Get the signal strength of the currently connected Wi-Fi network.
    pub async fn signal(&self) -> nmrs::Result<Option<ConnectedWifiSignal>> {
        let Some(network) = self.manager.current_network().await? else {
            return Ok(None);
        };

        Ok(Some(ConnectedWifiSignal {
            strength: network.strength.unwrap_or(0),
            secured: network.secured,
        }))
    }
}
