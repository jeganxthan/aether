use std::sync::Arc;

use zbus::{interface, object_server::SignalEmitter};

use super::manager::WifiManager;

pub struct WifiService {
    manager: Arc<WifiManager>,
}

impl WifiService {
    pub fn new(manager: Arc<WifiManager>) -> Self {
        Self { manager }
    }
}

#[interface(name = "org.aether.Wifi")]
impl WifiService {
    async fn is_wifi_enabled(&self) -> zbus::fdo::Result<bool> {
        self.manager
            .is_wifi_enabled()
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    async fn set_wifi_enabled(
        &self,
        enabled: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let current = self
            .manager
            .is_wifi_enabled()
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        if current == enabled {
            return Ok(());
        }

        self.manager
            .set_wifi_enabled(enabled)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        Self::wifi_state_changed(&emitter, enabled)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        Ok(())
    }

    async fn scan_networks(&self) -> zbus::fdo::Result<()> {
        self.manager
            .scan()
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    async fn list_networks(&self) -> zbus::fdo::Result<Vec<(String, u8, bool)>> {
        let networks = self
            .manager
            .list_access_points()
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        let networks = networks
            .into_iter()
            .map(|network| {
                (
                    network.access_point.ssid,
                    network.access_point.strength,
                    network.is_saved,
                )
            })
            .collect();

        Ok(networks)
    }

    async fn connected_wifi(
        &self,
    ) -> zbus::fdo::Result<Vec<(String, String, String, u8, String, String, String, String)>> {
        let devices = self
            .manager
            .connected_wifi()
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        if devices.is_empty() {
            return Ok(Vec::new());
        }

        let password_info = self
            .manager
            .connected_wifi_qr()
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        let mut result = Vec::new();

        for device in devices {
            if let nmrs::ActiveConnection::Wifi(wifi) = device.connection {
                result.push((
                    wifi.ssid,
                    wifi.interface.unwrap_or_default(),
                    wifi.bssid.unwrap_or_default(),
                    wifi.strength.unwrap_or_default(),
                    wifi.ip4_address.unwrap_or_default(),
                    wifi.ip6_address.unwrap_or_default(),
                    password_info.password.clone().unwrap_or_default(),
                    password_info.qr_payload.clone(),
                ));
            }
        }
        Ok(result)
    }

    async fn connect_network(
        &self,
        ssid: String,
        password: String,
        interface: String,
    ) -> zbus::fdo::Result<()> {
        self.manager
            .connect_wifi(
                &ssid,
                if password.is_empty() {
                    None
                } else {
                    Some(&password)
                },
                if interface.is_empty() {
                    None
                } else {
                    Some(&interface)
                },
            )
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        Ok(())
    }

    #[zbus(signal)]
    async fn wifi_state_changed(emitter: &SignalEmitter<'_>, enabled: bool) -> zbus::Result<()>;
}
