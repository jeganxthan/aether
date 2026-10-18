#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiState {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiStatus {
    pub state: WifiState,
    pub connected: bool,
    pub ssid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiAccessPoint {
    pub ssid: String,
    pub strength: u8,
    pub frequency_mhz: u32,
    pub bssid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedWifi {
    pub ssid: String,
    pub bssid: String,
    pub interface: String,
    pub connection_name: String,
    pub signal_strength: u8,
    pub frequency_mhz: u32,
    pub ipv4_address: Option<String>,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
}
