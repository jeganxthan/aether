use std::fs;
use std::path::Path;

use crate::models::{
    BatteryState,
    BatteryStatus,
};

const POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";

pub struct BatteryService;

impl BatteryService {
    pub fn new() -> Self {
        Self
    }

    pub fn status(&self) -> Result<BatteryStatus, String> {
        let battery_path = self.find_battery()?;

        let percentage = self.read_percentage(&battery_path)?;

        let state = self.read_state(&battery_path);

        let charging = matches!(
            state,
            BatteryState::Charging
        );

        let plugged_in =
            charging ||
            matches!(state, BatteryState::Full);

        Ok(BatteryStatus {
            percentage,
            state,
            charging,
            plugged_in,
        })
    }

    fn find_battery(&self) -> Result<String, String> {
        let entries = fs::read_dir(POWER_SUPPLY_PATH)
            .map_err(|error| {
                format!(
                    "Failed to read power supply: {}",
                    error
                )
            })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                error.to_string()
            })?;

            let name = entry.file_name();

            let name = name
                .to_string_lossy()
                .to_string();

            if !name.starts_with("BAT") {
                continue;
            }

            let battery_path =
                format!(
                    "{}/{}",
                    POWER_SUPPLY_PATH,
                    name
                );

            let battery_type = read_file(
                &format!(
                    "{}/type",
                    battery_path
                ),
            );

            if battery_type
                .as_deref()
                .map(|value| value.trim() == "Battery")
                .unwrap_or(true)
            {
                return Ok(battery_path);
            }
        }

        Err("No battery found".to_string())
    }

    fn read_percentage(
        &self,
        battery_path: &str,
    ) -> Result<u8, String> {
        let path =
            format!("{}/capacity", battery_path);

        let value = read_file(&path)
            .ok_or_else(|| {
                "Battery capacity is unavailable"
                    .to_string()
            })?;

        value
            .trim()
            .parse::<u8>()
            .map_err(|error| {
                format!(
                    "Invalid battery percentage: {}",
                    error
                )
            })
    }

    fn read_state(
        &self,
        battery_path: &str,
    ) -> BatteryState {
        let path =
            format!("{}/status", battery_path);

        let Some(value) = read_file(&path) else {
            return BatteryState::Unknown;
        };

        match value.trim().to_lowercase().as_str() {
            "charging" => BatteryState::Charging,

            "discharging" => {
                BatteryState::Discharging
            }

            "full" => BatteryState::Full,

            _ => BatteryState::Unknown,
        }
    }
}

fn read_file(path: &str) -> Option<String> {
    let path = Path::new(path);

    fs::read_to_string(path).ok()
}