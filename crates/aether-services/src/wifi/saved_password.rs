use std::process::Command;

#[derive(Debug)]
pub enum WifiPasswordError {
    CommandFailed(String),
    Utf8Error(std::string::FromUtf8Error),
    PasswordNotFound,
}

impl std::fmt::Display for WifiPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandFailed(message) => write!(f, "{message}"),
            Self::Utf8Error(error) => write!(f, "Invalid command output: {error}"),
            Self::PasswordNotFound => write!(f, "No saved Wi-Fi password found"),
        }
    }
}

impl std::error::Error for WifiPasswordError {}

/// Retrieves a saved Wi-Fi password from NetworkManager.
///
/// `connection_id` can be the connection name or UUID.
/// Call this only when the user explicitly requests the password.
pub fn get_saved_wifi_password(connection_id: &str) -> Result<String, WifiPasswordError> {
    if connection_id.trim().is_empty() {
        return Err(WifiPasswordError::CommandFailed(
            "Connection ID cannot be empty".into(),
        ));
    }

    let output = Command::new("nmcli")
        .args([
            "--show-secrets",
            "-g",
            "802-11-wireless-security.psk",
            "connection",
            "show",
            connection_id,
        ])
        .output()
        .map_err(|error| {
            WifiPasswordError::CommandFailed(format!("Failed to execute nmcli: {error}"))
        })?;

    if !output.status.success() {
        // Do not include command output that could contain sensitive data.
        return Err(WifiPasswordError::CommandFailed(
            "Could not read the saved connection. Check its ID and permissions.".into(),
        ));
    }

    let password = String::from_utf8(output.stdout)
        .map_err(WifiPasswordError::Utf8Error)?
        .trim()
        .to_owned();

    if password.is_empty() {
        return Err(WifiPasswordError::PasswordNotFound);
    }

    Ok(password)
}
