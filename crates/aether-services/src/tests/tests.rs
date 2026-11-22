use super::manager::WifiManager;
use super::model::{
    WifiConnectionStatus,
    WifiSecurityType,
};

// ------------------------------------------
// Security mapping tests
// ------------------------------------------

#[test]
fn test_security_open() {
    assert_eq!(
        WifiManager::map_security(false, false, false),
        WifiSecurityType::Open
    );
}

#[test]
fn test_security_wpa_psk() {
    assert_eq!(
        WifiManager::map_security(true, true, false),
        WifiSecurityType::WpaPsk
    );
}

#[test]
fn test_security_enterprise() {
    assert_eq!(
        WifiManager::map_security(true, false, true),
        WifiSecurityType::Enterprise
    );
}

#[test]
fn test_security_unknown() {
    assert_eq!(
        WifiManager::map_security(true, false, false),
        WifiSecurityType::Unknown
    );
}

// ------------------------------------------
// Frequency conversion tests
// ------------------------------------------

#[test]
fn test_frequency_2_4ghz() {
    assert_eq!(
        WifiManager::frequency_to_channel(2412),
        Some(1)
    );

    assert_eq!(
        WifiManager::frequency_to_channel(2437),
        Some(6)
    );

    assert_eq!(
        WifiManager::frequency_to_channel(2462),
        Some(11)
    );

    assert_eq!(
        WifiManager::frequency_to_channel(2484),
        Some(14)
    );
}

#[test]
fn test_frequency_5ghz() {
    assert_eq!(
        WifiManager::frequency_to_channel(5180),
        Some(36)
    );

    assert_eq!(
        WifiManager::frequency_to_channel(5200),
        Some(40)
    );

    assert_eq!(
        WifiManager::frequency_to_channel(5745),
        Some(149)
    );
}

#[test]
fn test_frequency_6ghz() {
    assert_eq!(
        WifiManager::frequency_to_channel(5955),
        Some(1)
    );

    assert_eq!(
        WifiManager::frequency_to_channel(5975),
        Some(5)
    );
}

#[test]
fn test_invalid_frequency() {
    assert_eq!(
        WifiManager::frequency_to_channel(3000),
        None
    );

    assert_eq!(
        WifiManager::frequency_to_channel(0),
        None
    );
}

// ------------------------------------------
// Model tests
// ------------------------------------------

#[test]
fn test_connection_status() {
    assert_ne!(
        WifiConnectionStatus::Connected,
        WifiConnectionStatus::Disconnected
    );
}

#[test]
fn test_security_types_are_distinct() {
    assert_ne!(
        WifiSecurityType::Open,
        WifiSecurityType::WpaPsk
    );

    assert_ne!(
        WifiSecurityType::Enterprise,
        WifiSecurityType::Unknown
    );
}

// ------------------------------------------
// NetworkManager integration tests
// ------------------------------------------

#[tokio::test]
#[ignore = "Requires NetworkManager"]
async fn test_wifi_manager_initialization() {
    let manager = WifiManager::new().await;

    assert!(
        manager.is_ok(),
        "Failed to connect to NetworkManager"
    );
}

#[tokio::test]
#[ignore = "Requires NetworkManager and a Wi-Fi adapter"]
async fn test_available_networks() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let networks = manager
        .available_networks()
        .await
        .expect("Failed to retrieve available networks");

    println!("Discovered {} networks", networks.len());

    for network in &networks {
        println!("{:#?}", network);

        assert!(!network.ssid.is_empty());
        assert!(network.strength <= 100);
    }
}

#[tokio::test]
#[ignore = "Requires NetworkManager and a Wi-Fi adapter"]
async fn test_scan_networks() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let networks = manager
        .scan_networks()
        .await
        .expect("Wi-Fi scan failed");

    println!("Found {} Wi-Fi networks", networks.len());

    for network in &networks {
        println!(
            "SSID: {} | Signal: {}% | Channel: {:?} | Security: {:?} | Status: {:?}",
            network.ssid,
            network.strength,
            network.channel,
            network.security,
            network.status,
        );
    }
}