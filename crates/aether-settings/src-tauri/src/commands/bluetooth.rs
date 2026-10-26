use serde::Serialize;
use std::{
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Debug, Clone, Serialize)]
pub struct BluetoothStatus {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BluetoothDevice {
    pub path: String,
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub rssi: Option<i16>,
}

fn run_bluetoothctl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("bluetoothctl")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Could not run bluetoothctl: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn bluetooth_is_powered() -> Result<bool, String> {
    let output = run_bluetoothctl(&["show"])?;

    for line in output.lines() {
        let line = line.trim();

        if let Some(value) = line.strip_prefix("Powered:") {
            return match value.trim() {
                "yes" => Ok(true),
                "no" => Ok(false),
                other => Err(format!("Unexpected Bluetooth power state: {other}")),
            };
        }
    }

    Err("Bluetooth adapter state was not found".into())
}

#[tauri::command]
pub fn bluetooth_status() -> Result<BluetoothStatus, String> {
    Ok(BluetoothStatus {
        enabled: bluetooth_is_powered()?,
    })
}

#[tauri::command]
pub fn bluetooth_set_enabled(enabled: bool) -> Result<bool, String> {
    let state = if enabled { "on" } else { "off" };

    run_bluetoothctl(&["power", state])?;

    // Read back the actual state instead of assuming success.
    bluetooth_is_powered()
}

#[tauri::command]
pub fn bluetooth_devices() -> Result<Vec<BluetoothDevice>, String> {
    let output = run_bluetoothctl(&["devices", "Paired"])?;

    let mut devices = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        let Some(rest) = line.strip_prefix("Device ") else {
            continue;
        };

        let Some((address, name)) = rest.split_once(' ') else {
            continue;
        };

        // Query each device's properties.
        let info = match run_bluetoothctl(&["info", address]) {
            Ok(info) => info,
            Err(_) => continue,
        };

        let property = |key: &str| -> bool {
            info.lines().any(|line| {
                let line = line.trim();
                line.strip_prefix(key)
                    .map(|value| value.trim() == "yes")
                    .unwrap_or(false)
            })
        };

        let device_path = format!("/org/bluez/hci0/dev_{}", address.replace(':', "_"));

        devices.push(BluetoothDevice {
            path: device_path,
            address: address.to_string(),
            name: name.to_string(),
            paired: property("Paired:"),
            connected: property("Connected:"),
            trusted: property("Trusted:"),
            rssi: None,
        });
    }

    Ok(devices)
}

#[tauri::command]
pub fn bluetooth_start_scan() -> Result<(), String> {
    run_bluetoothctl(&["scan", "on"])?;
    Ok(())
}

#[tauri::command]
pub fn bluetooth_stop_scan() -> Result<(), String> {
    run_bluetoothctl(&["scan", "off"])?;
    Ok(())
}

fn validate_address(address: &str) -> Result<(), String> {
    let parts: Vec<&str> = address.split(':').collect();

    if parts.len() != 6
        || parts
            .iter()
            .any(|part| part.len() != 2 || !part.chars().all(|ch| ch.is_ascii_hexdigit()))
    {
        return Err("Invalid Bluetooth device address".into());
    }

    Ok(())
}

fn address_from_path(path: &str) -> Result<String, String> {
    let device = path
        .strip_prefix("/org/bluez/hci0/dev_")
        .ok_or_else(|| "Invalid Bluetooth device path".to_string())?;

    let address = device.replace('_', ":");
    validate_address(&address)?;
    Ok(address)
}

#[tauri::command]
pub fn bluetooth_pair(path: String) -> Result<(), String> {
    let address = address_from_path(&path)?;
    run_bluetoothctl(&["pair", &address])?;
    Ok(())
}

#[tauri::command]
pub fn bluetooth_connect(path: String) -> Result<(), String> {
    let address = address_from_path(&path)?;
    run_bluetoothctl(&["connect", &address])?;
    Ok(())
}

#[tauri::command]
pub fn bluetooth_disconnect(path: String) -> Result<(), String> {
    let address = address_from_path(&path)?;
    run_bluetoothctl(&["disconnect", &address])?;
    Ok(())
}
