use nmrs::{AccessPoint as NetworkManagerAccessPoint, ActiveConnection};

#[derive(Debug, Clone)]
pub struct WifiAccessPoint {
    pub access_point: NetworkManagerAccessPoint,
    pub is_saved: bool,
}

#[derive(Debug, Clone)]
pub struct ConnectedDevice {
    pub connection: ActiveConnection,
}

pub struct WifiQrInfo {
    pub ssid: String,
    pub password: Option<String>,
    pub qr_payload: String,
    pub terminal_qr: String,
}
