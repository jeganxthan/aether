use nmrs::{ActiveConnection, NetworkManager, SettingsSummary};

use super::model::WifiAccessPoint;
use crate::services::network::wifi::model::{ConnectedDevice, WifiQrInfo};
use async_process::Command;
use std::{process::Stdio, time::Duration};

use futures_lite::io::{AsyncReadExt, AsyncWriteExt};

pub struct WifiManager {
    network_manager: NetworkManager,
}

impl WifiManager {
    pub async fn new() -> nmrs::Result<Self> {
        let network_manager = NetworkManager::new().await?;

        Ok(Self { network_manager })
    }

    pub async fn scan(&self) -> nmrs::Result<()> {
        for attempt in 0..=3 {
            match self.network_manager.scan_networks(None).await {
                Err(error) if attempt < 3 && is_device_churn_error(&error.to_string()) => {
                    tokio::time::sleep(Duration::from_millis(250 * (attempt + 1))).await;
                }
                result => return result,
            }
        }

        unreachable!("the scan retry loop always returns its final result")
    }

    pub async fn list_access_points(&self) -> nmrs::Result<Vec<WifiAccessPoint>> {
        for attempt in 0..=3 {
            match self.list_access_points_once().await {
                Err(error) if attempt < 3 && is_device_churn_error(&error.to_string()) => {
                    tokio::time::sleep(Duration::from_millis(250 * (attempt + 1))).await;
                }
                result => return result,
            }
        }

        unreachable!("the access point retry loop always returns its final result")
    }

    async fn list_access_points_once(&self) -> nmrs::Result<Vec<WifiAccessPoint>> {
        let (access_points, saved_connections) = tokio::try_join!(
            self.network_manager.list_access_points(None),
            self.network_manager.list_saved_connections(),
        )?;

        let mut networks: Vec<WifiAccessPoint> = access_points
            .into_iter()
            .map(|access_point| {
                let is_saved = saved_connections.iter().any(|connection| {
                    matches!(
                        &connection.summary,
                        SettingsSummary::Wifi { ssid, .. }
                            if ssid == &access_point.ssid
                    )
                });

                WifiAccessPoint {
                    access_point,
                    is_saved,
                }
            })
            .collect();

        // Strongest access points first.
        networks.sort_by(|a, b| b.access_point.strength.cmp(&a.access_point.strength));

        Ok(networks)
    }

    pub async fn connected_wifi(&self) -> nmrs::Result<Vec<ConnectedDevice>> {
        let active_connections = self.network_manager.list_active_connections().await?;

        let connected_wifi = active_connections
            .into_iter()
            .filter(|connection| matches!(connection, ActiveConnection::Wifi(_)))
            .map(|connection| ConnectedDevice { connection })
            .collect();

        Ok(connected_wifi)
    }

    pub async fn connected_wifi_qr(&self) -> Result<WifiQrInfo, String> {
        // 1. Find active Wi-Fi connection profiles.
        let active_output = run_nmcli(&[
            "--terse".into(),
            "--escape".into(),
            "no".into(),
            "--fields".into(),
            "UUID,TYPE".into(),
            "connection".into(),
            "show".into(),
            "--active".into(),
        ])
        .await?;

        let uuid = active_output
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(_, connection_type)| *connection_type == "802-11-wireless")
            .map(|(uuid, _)| uuid.to_string())
            .ok_or_else(|| "No active Wi-Fi connection found".to_string())?;

        // 2. Retrieve SSID.
        let ssid = get_connection_property(&uuid, "802-11-wireless.ssid").await?;

        if ssid.is_empty() {
            return Err("Connected Wi-Fi SSID is empty".into());
        }

        // 3. Retrieve the security type.
        let key_mgmt = get_connection_property(&uuid, "802-11-wireless-security.key-mgmt").await?;

        // 4. Retrieve the saved password.
        let password = get_connection_property(&uuid, "802-11-wireless-security.psk").await?;

        // 5. Build the Wi-Fi QR payload.
        let (password, qr_payload) = match key_mgmt.as_str() {
            "" | "none" => (
                None,
                format!("WIFI:T:nopass;S:{};;", escape_wifi_field(&ssid)),
            ),

            "wpa-psk" | "sae" | "wpa-psk sae" => {
                if password.is_empty() {
                    return Err("No saved Wi-Fi password is available".into());
                }

                let payload = format!(
                    "WIFI:T:WPA;S:{};P:{};;",
                    escape_wifi_field(&ssid),
                    escape_wifi_field(&password),
                );

                (Some(password), payload)
            }

            other => {
                return Err(format!("Unsupported Wi-Fi security type: {other}"));
            }
        };

        // 6. Generate the QR code in the terminal.
        let terminal_qr = generate_terminal_qr(&qr_payload).await?;

        Ok(WifiQrInfo {
            ssid,
            password,
            qr_payload,
            terminal_qr,
        })
    }

    pub async fn set_wifi_enabled(&self, enabled: bool) -> nmrs::Result<()> {
        self.network_manager.set_wireless_enabled(enabled).await
    }

    pub async fn is_wifi_enabled(&self) -> nmrs::Result<bool> {
        let state = self.network_manager.wifi_state().await?;
        Ok(state.enabled)
    }

    pub async fn is_wifi_hardware_enabled(&self) -> nmrs::Result<bool> {
        let state = self.network_manager.wifi_state().await?;

        Ok(state.hardware_enabled)
    }

    pub async fn connect_saved_wifi(&self, uuid: &str) -> Result<(), String> {
        validate_identifier(uuid)?;

        run_nmcli(&["connection".into(), "up".into(), "uuid".into(), uuid.into()]).await?;

        Ok(())
    }

    pub async fn connect_wifi(
        &self,
        ssid: &str,
        password: Option<&str>,
        interface: Option<&str>,
    ) -> Result<(), String> {
        if ssid.trim().is_empty() {
            return Err("Wi-Fi SSID cannot be empty".into());
        }

        let mut args = vec![
            "--wait".into(),
            "30".into(),
            "device".into(),
            "wifi".into(),
            "connect".into(),
            ssid.into(),
        ];

        if let Some(password) = password {
            if password.is_empty() {
                return Err("Wi-Fi password cannot be empty".into());
            }

            args.push("password".into());
            args.push(password.into());
        }

        if let Some(interface) = interface {
            if interface.trim().is_empty() {
                return Err("Wi-Fi interface cannot be empty".into());
            }

            args.push("ifname".into());
            args.push(interface.into());
        }

        run_nmcli(&args).await?;

        Ok(())
    }

    pub async fn disconnect_wifi(&self, uuid: &str) -> Result<(), String> {
        validate_identifier(uuid)?;

        run_nmcli(&[
            "connection".into(),
            "down".into(),
            "uuid".into(),
            uuid.into(),
        ])
        .await?;

        Ok(())
    }

    pub async fn forget_wifi(&self, uuid: &str) -> Result<(), String> {
        validate_identifier(uuid)?;

        run_nmcli(&[
            "connection".into(),
            "delete".into(),
            "uuid".into(),
            uuid.into(),
        ])
        .await?;

        Ok(())
    }
}

fn is_device_churn_error(message: &str) -> bool {
    message.contains("NetworkManager/Devices/")
        && (message.contains("UnknownMethod")
            || message.contains("UnknownObject")
            || message.contains("object does not exist")
            || message.contains("Object does not exist"))
}

#[test]
fn retries_only_transient_networkmanager_device_errors() {
    assert!(is_device_churn_error(
        "failed to get device type for /org/freedesktop/NetworkManager/Devices/11 during Wi-Fi scan: org.freedesktop.DBus.Error.UnknownMethod"
    ));
    assert!(!is_device_churn_error(
        "org.freedesktop.DBus.Error.Failed: permission denied"
    ));
}

async fn run_nmcli(args: &[String]) -> Result<String, String> {
    let output = Command::new("nmcli")
        .args(args)
        .output()
        .await
        .map_err(|error| format!("Failed to execute nmcli: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "nmcli failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\r', '\n'])
        .to_string())
}

async fn get_connection_property(uuid: &str, property: &str) -> Result<String, String> {
    let args = vec![
        "--escape".into(),
        "no".into(),
        "--show-secrets".into(),
        "--get-values".into(),
        property.into(),
        "connection".into(),
        "show".into(),
        "uuid".into(),
        uuid.into(),
    ];

    run_nmcli(&args).await
}

fn escape_wifi_field(value: &str) -> String {
    let mut escaped = String::new();

    for character in value.chars() {
        if matches!(character, '\\' | ';' | ',' | ':' | '"') {
            escaped.push('\\');
        }

        escaped.push(character);
    }

    escaped
}

fn validate_identifier(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("Connection UUID cannot be empty".into());
    }

    Ok(())
}

async fn generate_terminal_qr(payload: &str) -> Result<String, String> {
    let mut child = Command::new("qrencode")
        .args(["-t", "ANSIUTF8"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to execute qrencode. Install qrencode: {error}"))?;

    // Write the QR payload to the child process.
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Could not open qrencode stdin".to_string())?;

    stdin
        .write_all(payload.as_bytes())
        .await
        .map_err(|error| format!("Failed to write QR payload: {error}"))?;

    // Close stdin so qrencode knows that input is complete.
    drop(stdin);

    // Capture stdout and stderr.
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not open qrencode stdout".to_string())?;

    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Could not open qrencode stderr".to_string())?;

    let mut stdout_text = String::new();
    let mut stderr_text = String::new();

    // Read both streams concurrently to avoid pipe deadlocks.
    let (stdout_result, stderr_result) = futures_lite::future::zip(
        stdout.read_to_string(&mut stdout_text),
        stderr.read_to_string(&mut stderr_text),
    )
    .await;

    stdout_result.map_err(|error| format!("Failed to read QR output: {error}"))?;

    stderr_result.map_err(|error| format!("Failed to read QR errors: {error}"))?;

    // Wait for the process to finish.
    let status = child
        .status()
        .await
        .map_err(|error| format!("QR generation failed: {error}"))?;

    if !status.success() {
        return Err(format!("qrencode failed: {}", stderr_text.trim()));
    }

    Ok(stdout_text)
}

#[tokio::test]
#[ignore = "Requires NetworkManager and a working Wi-Fi adapter"]
async fn test_wifi_scan() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let result = manager.scan().await;

    assert!(result.is_ok(), "Wi-Fi scan failed: {:?}", result.err());
}

#[tokio::test]
#[ignore = "Requires NetworkManager and a working Wi-Fi adapter"]
async fn test_list_access_points() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let access_points = manager
        .list_access_points()
        .await
        .expect("Failed to list access points");

    println!("Available Wi-Fi networks: {}", access_points.len());

    for (index, network) in access_points.iter().enumerate() {
        println!("\n{}. {}", index + 1, network.access_point.ssid);
        println!("   Signal: {}%", network.access_point.strength);
        println!("   Saved: {}", network.is_saved);
        println!("   Access Point: {:#?}", network.access_point);
    }
}

#[tokio::test]
#[ignore = "Requires NetworkManager and a working Wi-Fi adapter"]
async fn test_connected_wifi() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let connected_devices = manager
        .connected_wifi()
        .await
        .expect("Failed to list connected Wi-Fi networks");

    println!("Connected Wi-Fi networks: {}", connected_devices.len());

    for (index, device) in connected_devices.iter().enumerate() {
        println!("\n{}. Connected Wi-Fi", index + 1);

        match &device.connection {
            nmrs::ActiveConnection::Wifi(wifi) => {
                println!("SSID: {}", wifi.ssid);
                println!("Interface: {:?}", wifi.interface);
                println!("BSSID: {:?}", wifi.bssid);
                println!("Signal Strength: {:?}", wifi.strength);
                println!("IPv4 Address: {:?}", wifi.ip4_address);
                println!("IPv6 Address: {:?}", wifi.ip6_address);
            }
            _ => unreachable!("connected_wifi should only return Wi-Fi connections"),
        }
    }
}

#[tokio::test]
#[ignore = "Requires NetworkManager, an active Wi-Fi connection, and qrencode"]
async fn test_connected_wifi_qr() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let wifi = manager
        .connected_wifi_qr()
        .await
        .expect("Failed to retrieve connected Wi-Fi credentials");

    assert!(!wifi.ssid.is_empty());
    assert!(!wifi.qr_payload.is_empty());
    assert!(!wifi.terminal_qr.is_empty());

    println!("\nConnected Wi-Fi: {}", wifi.ssid);
    println!("\nWi-Fi password: {:?}", wifi.password);

    if wifi.password.is_some() {
        println!("Saved password retrieved successfully");
    } else {
        println!("Open Wi-Fi network");
    }

    println!("\nWi-Fi QR Code:\n");
    println!("{}", wifi.terminal_qr);
}

#[tokio::test]
#[ignore = "Requires NetworkManager"]
async fn test_wifi_status() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let enabled = manager
        .is_wifi_enabled()
        .await
        .expect("Failed to get Wi-Fi state");

    let hardware_enabled = manager
        .is_wifi_hardware_enabled()
        .await
        .expect("Failed to get Wi-Fi hardware state");

    println!("Wi-Fi enabled: {enabled}");
    println!("Wi-Fi hardware enabled: {hardware_enabled}");
}

#[tokio::test]
#[ignore = "Changes actual system Wi-Fi radio state"]
async fn test_wifi_toggle() {
    let manager = WifiManager::new()
        .await
        .expect("Failed to initialize NetworkManager");

    let original_state = manager
        .is_wifi_enabled()
        .await
        .expect("Failed to get original Wi-Fi state");

    // Turn Wi-Fi OFF.
    manager
        .set_wifi_enabled(false)
        .await
        .expect("Failed to disable Wi-Fi");

    let disabled = manager
        .is_wifi_enabled()
        .await
        .expect("Failed to check disabled state");

    assert!(!disabled, "Wi-Fi should be disabled");

    // Turn Wi-Fi ON.
    manager
        .set_wifi_enabled(true)
        .await
        .expect("Failed to enable Wi-Fi");

    let enabled = manager
        .is_wifi_enabled()
        .await
        .expect("Failed to check enabled state");

    assert!(enabled, "Wi-Fi should be enabled");

    // Restore original state.
    manager
        .set_wifi_enabled(original_state)
        .await
        .expect("Failed to restore original Wi-Fi state");
}
