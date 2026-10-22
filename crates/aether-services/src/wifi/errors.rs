use nmrs::ConnectionError;

#[derive(Debug)]
pub enum WifiError {
    InvalidSsid,
    InvalidPassword,
    AuthenticationFailed,
    NetworkNotFound,
    ConnectionTimedOut,
    DhcpFailed,
    MissingPassword,
    HardwareRadioBlocked,
    OperationInProgress,
    NetworkManagerUnavailable,
    OperationFailed(String),
}

impl WifiError {
    pub fn from_nmrs(error: ConnectionError) -> Self {
        match error {
            ConnectionError::AuthFailed => Self::AuthenticationFailed,
            ConnectionError::NotFound => Self::NetworkNotFound,
            ConnectionError::Timeout => Self::ConnectionTimedOut,
            ConnectionError::DhcpFailed => Self::DhcpFailed,
            ConnectionError::MissingPassword => Self::MissingPassword,
            ConnectionError::HardwareRadioKilled => Self::HardwareRadioBlocked,
            other => Self::OperationFailed(other.to_string()),
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidSsid => "Enter a valid Wi-Fi network name.",
            Self::InvalidPassword => "The password format is invalid.",
            Self::AuthenticationFailed => "Authentication failed. Check the password.",
            Self::NetworkNotFound => "The Wi-Fi network was not found nearby.",
            Self::ConnectionTimedOut => "The connection timed out.",
            Self::DhcpFailed => "Could not obtain an IP address.",
            Self::MissingPassword => "This network requires a password.",
            Self::HardwareRadioBlocked => "Wi-Fi is blocked by a hardware switch.",
            Self::OperationInProgress => "A Wi-Fi operation is already in progress.",
            Self::NetworkManagerUnavailable => "NetworkManager is unavailable.",
            Self::OperationFailed(_) => "The Wi-Fi operation failed.",
        }
    }
}
