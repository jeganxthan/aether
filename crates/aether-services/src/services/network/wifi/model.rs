
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct NetworkInfo {
    // Network identity
    pub ssid: String,
    pub mac_address: Option<String>,

    // Signal information
    pub strength: u8,

    // Radio information
    pub freq: Option<u32>,
    pub channel: Option<u16>,
    pub mode: WifiMode,

    // Connection information
    pub connection_speed_mbps: Option<u32>,
    pub status: WifiConnectionStatus,

    // Security information
    pub security: WifiSecurityType,

    // Device information
    pub device: Option<String>,

    // IP information
    pub ip4_address: Option<String>,
    pub ip6_address: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum WifiSecurityType {
    Open,
    Wep,
    WpaPsk,
    Wpa2Psk,
    Wpa3Sae,
    Enterprise,
    Unknown,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum WifiConnectionStatus {
    Connected,
    Disconnected,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum WifiMode {
    Infrastructure,
    AdHoc,
    AccessPoint,
    Unknown,
}
