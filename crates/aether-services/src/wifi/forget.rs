use nmrs::NetworkManager;

#[derive(Debug)]
pub enum ForgetWifiError {
    EmptySsid,
    NetworkManager(nmrs::ConnectionError),
}

impl std::fmt::Display for ForgetWifiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySsid => write!(f, "SSID cannot be empty"),
            Self::NetworkManager(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ForgetWifiError {}

pub async fn forget_saved_network(
    manager: &NetworkManager,
    ssid: &str,
) -> Result<(), ForgetWifiError> {
    if ssid.trim().is_empty() {
        return Err(ForgetWifiError::EmptySsid);
    }

    manager
        .forget(ssid)
        .await
        .map_err(ForgetWifiError::NetworkManager)
}
