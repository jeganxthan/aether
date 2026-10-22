use nmrs::NetworkManager;

#[derive(Debug, Clone)]
pub struct SavedWifiNetwork {
    pub id: String,
    pub uuid: String,
    pub interface_name: Option<String>,
    pub autoconnect: bool,
}

/// Returns all saved Wi-Fi connection profiles.
pub async fn list_saved_wifi_networks(
    manager: &NetworkManager,
) -> nmrs::Result<Vec<SavedWifiNetwork>> {
    let connections = manager.list_saved_connections().await?;

    let networks = connections
        .into_iter()
        .filter(|connection| connection.connection_type == "802-11-wireless")
        .map(|connection| SavedWifiNetwork {
            id: connection.id,
            uuid: connection.uuid,
            interface_name: connection.interface_name,
            autoconnect: connection.autoconnect,
        })
        .collect();

    Ok(networks)
}
