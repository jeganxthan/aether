use std::process::Command;

use crate::{
    error::AetherError,
    models::{WifiNetwork, WifiStatus},
};

pub struct WifiService;

impl WifiService {
    pub fn new() -> Self {
        Self
    }

    fn run_nmcli(&self, args: &[&str]) -> Result<String, AetherError> {
        let output = Command::new("nmcli").args(args).output().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AetherError::CommandNotFound("nmcli (NetworkManager)".to_string())
            } else {
                AetherError::CommandFailed(error.to_string())
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            return Err(AetherError::CommandFailed(stderr.trim().to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn status(&self) -> Result<WifiStatus, AetherError> {
        let radio = self.run_nmcli(&["radio", "wifi"])?;

        if radio.trim() != "enabled" {
            return Ok(WifiStatus {
                enabled: false,
                connected: false,
                ssid: None,
                signal: None,
            });
        }

        let output = self.run_nmcli(&[
            "-t",
            "-f",
            "DEVICE,TYPE,STATE,CONNECTION",
            "device",
            "status",
        ])?;

        for line in output.lines() {
            let parts = parse_nmcli_line(line);

            if parts.len() < 4 {
                continue;
            }

            let device = &parts[0];
            let device_type = &parts[1];
            let state = &parts[2];
            let connection = &parts[3];

            if device_type != "wifi" {
                continue;
            }

            let connected = state == "connected";

            if !connected {
                return Ok(WifiStatus {
                    enabled: true,
                    connected: false,
                    ssid: None,
                    signal: None,
                });
            }

            let signal_output =
                self.run_nmcli(&["-t", "-f", "ACTIVE,SSID,SIGNAL", "device", "wifi", "list"])?;

            for wifi_line in signal_output.lines() {
                let parts = parse_nmcli_line(wifi_line);

                if parts.len() < 3 {
                    continue;
                }

                let active = &parts[0];

                if active != "yes" {
                    continue;
                }

                let ssid = parts[1].clone();

                let signal = parts[2].parse::<u8>().ok();

                return Ok(WifiStatus {
                    enabled: true,
                    connected: true,
                    ssid: if ssid.is_empty() {
                        Some(connection.clone())
                    } else {
                        Some(ssid)
                    },
                    signal,
                });
            }

            return Ok(WifiStatus {
                enabled: true,
                connected: true,
                ssid: Some(connection.clone()),
                signal: None,
            });
        }

        Ok(WifiStatus {
            enabled: false,
            connected: false,
            ssid: None,
            signal: None,
        })
    }

    pub fn networks(&self) -> Result<Vec<WifiNetwork>, AetherError> {
        let output = self.run_nmcli(&[
            "-t",
            "-f",
            "IN-USE,SSID,SIGNAL,SECURITY",
            "device",
            "wifi",
            "list",
        ])?;

        self.parse_networks(&output)
    }

    pub fn enable(&self) -> Result<(), AetherError> {
        self.run_nmcli(&["radio", "wifi", "on"])?;
        Ok(())
    }

    pub fn disable(&self) -> Result<(), AetherError> {
        self.run_nmcli(&["radio", "wifi", "off"])?;
        Ok(())
    }

    pub fn rescan(&self) -> Result<(), AetherError> {
        self.run_nmcli(&["device", "wifi", "rescan"])?;
        Ok(())
    }

    pub fn connect(&self, ssid: &str, password: Option<&str>) -> Result<(), AetherError> {
        match password {
            Some(password) => {
                self.run_nmcli(&["device", "wifi", "connect", ssid, "password", password])?;
            }

            None => {
                self.run_nmcli(&["device", "wifi", "connect", ssid])?;
            }
        }

        Ok(())
    }

    fn wifi_device(&self) -> Result<String, AetherError> {
        let output = self.run_nmcli(&[
            "-t",
            "-f",
            "DEVICE,TYPE",
            "device",
            "status",
        ])?;

        for line in output.lines() {
            let parts = parse_nmcli_line(line);

            if parts.len() < 2 {
                continue;
            }

            if parts[1].trim() == "wifi" {
                return Ok(parts[0].trim().to_string());
            }
        }

        Err(AetherError::NetworkError(
            "No Wi-Fi interface found".into(),
        ))
    }

    pub fn disconnect(&self) -> Result<(), AetherError> {
        let status = self.status()?;

        if !status.connected {
            return Ok(());
        }

        let device = self.wifi_device()?;

        self.run_nmcli(&["device", "disconnect", &device])?;

        Ok(())
    }

    fn parse_status(&self, output: &str) -> Result<WifiStatus, AetherError> {
        let line = output.lines().next().unwrap_or_default();

        let parts: Vec<&str> = line.split(':').collect();

        if parts.len() < 4 {
            return Err(AetherError::ParseError(
                "Invalid NetworkManager status output".into(),
            ));
        }

        let enabled = parts[0] == "enabled";

        let connected = parts[1] == "connected";

        let ssid = if connected && !parts[2].is_empty() {
            Some(parts[2].to_string())
        } else {
            None
        };

        let signal = parts[3].parse::<u8>().ok();

        Ok(WifiStatus {
            enabled,
            connected,
            ssid,
            signal,
        })
    }

    fn parse_networks(&self, output: &str) -> Result<Vec<WifiNetwork>, AetherError> {
        let mut networks = Vec::new();

        for line in output.lines() {
            let parts = parse_nmcli_line(line);

            if parts.len() < 4 {
                continue;
            }

            let connected = parts[0].trim() == "*";

            let ssid = parts[1].trim();

            if ssid.is_empty() {
                continue;
            }

            let signal = parts[2].trim().parse::<u8>().unwrap_or(0);

            let security = parts[3].trim().to_string();

            networks.push(WifiNetwork {
                ssid: ssid.to_string(),
                signal,
                security,
                connected,
            });
        }

        Ok(networks)
    }
}
fn parse_nmcli_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match character {
            '\\' => {
                escaped = true;
            }

            ':' => {
                fields.push(current);
                current = String::new();
            }

            _ => {
                current.push(character);
            }
        }
    }

    fields.push(current);

    fields
}