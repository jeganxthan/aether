use zbus::{Connection, Proxy};

use super::types::{ConnectedWifi, WifiAccessPoint, WifiState, WifiStatus};

const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const NM_INTERFACE: &str = "org.freedesktop.NetworkManager";

pub struct WifiManager {
    connection: Connection,
}

impl WifiManager {
    pub async fn new() -> zbus::Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }
    pub async fn is_enabled(&self) -> zbus::Result<bool> {
        let proxy = Proxy::new(&self.connection, NM_SERVICE, NM_PATH, NM_INTERFACE).await?;

        proxy.get_property("WirelessEnabled").await
    }
    pub async fn status(&self) -> zbus::Result<WifiStatus> {
        let enabled = self.is_enabled().await?;

        Ok(WifiStatus {
            state: if enabled {
                WifiState::Enabled
            } else {
                WifiState::Disabled
            },
            connected: false,
            ssid: None,
        })
    }

    pub async fn available_networks(&self) -> zbus::Result<Vec<WifiAccessPoint>> {
        use zbus::zvariant::OwnedObjectPath;

        let device_proxy = Proxy::new(&self.connection, NM_SERVICE, NM_PATH, NM_INTERFACE).await?;

        let devices: Vec<OwnedObjectPath> = device_proxy.call("GetDevices", &()).await?;

        let mut networks = Vec::new();

        for device_path in devices {
            let device_proxy = Proxy::new(
                &self.connection,
                NM_SERVICE,
                device_path.as_str(),
                "org.freedesktop.NetworkManager.Device",
            )
            .await?;

            let device_type: u32 = device_proxy.get_property("DeviceType").await?;

            // NetworkManager uses device type 2 for Wi-Fi.
            if device_type != 2 {
                continue;
            }

            let wifi_proxy = Proxy::new(
                &self.connection,
                NM_SERVICE,
                device_path.as_str(),
                "org.freedesktop.NetworkManager.Device.Wireless",
            )
            .await?;

            let access_points: Vec<OwnedObjectPath> =
                wifi_proxy.call("GetAllAccessPoints", &()).await?;

            for ap_path in access_points {
                let ap_proxy = Proxy::new(
                    &self.connection,
                    NM_SERVICE,
                    ap_path.as_str(),
                    "org.freedesktop.NetworkManager.AccessPoint",
                )
                .await?;

                let ssid_bytes: Vec<u8> = ap_proxy.get_property("Ssid").await?;

                let strength: u8 = ap_proxy.get_property("Strength").await?;

                let frequency_mhz: u32 = ap_proxy.get_property("Frequency").await?;

                let bssid: String = ap_proxy.get_property("HwAddress").await?;

                networks.push(WifiAccessPoint {
                    ssid: String::from_utf8_lossy(&ssid_bytes).into_owned(),
                    strength,
                    frequency_mhz,
                    bssid,
                });
            }
        }

        Ok(networks)
    }

    pub async fn connected_wifi(&self) -> zbus::Result<Option<ConnectedWifi>> {
        use zbus::zvariant::OwnedObjectPath;

        let manager = Proxy::new(&self.connection, NM_SERVICE, NM_PATH, NM_INTERFACE).await?;

        let devices: Vec<OwnedObjectPath> = manager.call("GetDevices", &()).await?;

        for path in devices {
            let device = Proxy::new(
                &self.connection,
                NM_SERVICE,
                path.as_str(),
                "org.freedesktop.NetworkManager.Device",
            )
            .await?;

            let device_type: u32 = device.get_property("DeviceType").await?;

            if device_type != 2 {
                continue;
            }

            let active_path: OwnedObjectPath = device.get_property("ActiveConnection").await?;

            if active_path.as_str() == "/" {
                continue;
            }

            let interface: String = device.get_property("Interface").await?;

            let wifi = Proxy::new(
                &self.connection,
                NM_SERVICE,
                path.as_str(),
                "org.freedesktop.NetworkManager.Device.Wireless",
            )
            .await?;

            let ap_path: OwnedObjectPath = wifi.get_property("ActiveAccessPoint").await?;

            if ap_path.as_str() == "/" {
                continue;
            }

            let ap = Proxy::new(
                &self.connection,
                NM_SERVICE,
                ap_path.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
            )
            .await?;

            let ssid_bytes: Vec<u8> = ap.get_property("Ssid").await?;

            let ssid = String::from_utf8_lossy(&ssid_bytes).into_owned();

            let bssid: String = ap.get_property("HwAddress").await?;

            let signal_strength: u8 = ap.get_property("Strength").await?;

            let frequency_mhz: u32 = ap.get_property("Frequency").await?;

            let active = Proxy::new(
                &self.connection,
                NM_SERVICE,
                active_path.as_str(),
                "org.freedesktop.NetworkManager.Connection.Active",
            )
            .await?;

            let connection_name: String = active.get_property("Id").await?;

            let ip4_path: OwnedObjectPath = device.get_property("Ip4Config").await?;

            let mut ipv4_address = None;
            let mut gateway = None;
            let mut dns_servers = Vec::new();

            if ip4_path.as_str() != "/" {
                let ip4 = Proxy::new(
                    &self.connection,
                    NM_SERVICE,
                    ip4_path.as_str(),
                    "org.freedesktop.NetworkManager.IP4Config",
                )
                .await?;

                let addresses: Vec<Vec<u32>> = ip4.get_property("Addresses").await?;

                ipv4_address = addresses
                    .first()
                    .and_then(|entry| entry.first())
                    .map(|address| std::net::Ipv4Addr::from(address.to_be_bytes()).to_string());

                let gateway_value: String = ip4.get_property("Gateway").await?;

                gateway = if gateway_value.is_empty() {
                    None
                } else {
                    Some(gateway_value)
                };

                let nameservers: Vec<u32> = ip4.get_property("Nameservers").await?;

                dns_servers = nameservers
                    .into_iter()
                    .map(|ip| std::net::Ipv4Addr::from(ip.to_be_bytes()).to_string())
                    .collect();
            }

            return Ok(Some(ConnectedWifi {
                ssid,
                bssid,
                interface,
                connection_name,
                signal_strength,
                frequency_mhz,
                ipv4_address,
                gateway,
                dns_servers,
            }));
        }

        Ok(None)
    }

    pub async fn set_enabled(&self, enabled: bool) -> zbus::Result<()> {
        let proxy = Proxy::new(&self.connection, NM_SERVICE, NM_PATH, NM_INTERFACE).await?;

        proxy.call_method("Enable", &(enabled)).await?;

        Ok(())
    }

    pub async fn toggle(&self) -> zbus::Result<bool> {
        let current = self.is_enabled().await?;
        let requested = !current;

        self.set_enabled(requested).await?;

        Ok(requested)
    }

    pub async fn force_enable(&self) -> zbus::Result<()> {
        if !self.is_enabled().await? {
            match self.set_enabled(true).await {
                Ok(()) => {}
                Err(err) => {
                    if !self.is_enabled().await? {
                        return Err(err);
                    }
                }
            }
        }
        if !self.is_enabled().await? {
            return Err(zbus::Error::Failure(
                "NetworkManager did enabled wifi".into(),
            ));
        }
        Ok(())
    }
}
