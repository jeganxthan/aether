use std::sync::Arc;

use zbus::Connection;

use aether_services::services::{
    appearance::{
        dbus::AppearanceService,
        manager::ThemeManager,
    },
    network::wifi::{
        dbus::WifiService,
        manager::WifiManager,
    },
};

pub async fn start() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create the session D-Bus connection.
    let connection = Connection::session().await?;

    // Acquire the Aether bus name.
    connection
        .request_name("org.aether.Service")
        .await?;

    // Initialize Appearance.
    let theme_manager = ThemeManager::load()?;
    let appearance_service = AppearanceService::new(theme_manager);

    // Initialize Wi-Fi.
    let wifi_manager = Arc::new(WifiManager::new().await?);
    let wifi_service = WifiService::new(wifi_manager);

    // Register Appearance.
    connection
        .object_server()
        .at("/org/aether/Appearance", appearance_service)
        .await?;

    // Register Wi-Fi.
    connection
        .object_server()
        .at("/org/aether/Network/Wifi", wifi_service)
        .await?;

    println!("Aether services daemon started");

    tokio::signal::ctrl_c().await?;

    Ok(())
}