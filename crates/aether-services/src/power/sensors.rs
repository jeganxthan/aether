use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemperatureSensor {
    pub name: String,
    pub temperature_c: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SensorInfo {
    pub cpu_temperature_c: Option<f64>,
    pub temperatures: Vec<TemperatureSensor>,
}

pub fn read_sensors() -> SensorInfo {
    let root = Path::new("/sys/class/thermal");
    let mut temperatures = read_thermal_zones(root);
    temperatures.extend(read_hwmon_temperatures());

    let cpu_temperature_c = temperatures
        .iter()
        .find(|sensor| {
            let name = sensor.name.to_lowercase();
            name.contains("cpu")
                || name.contains("x86_pkg")
                || name.contains("tctl")
                || name.contains("soc")
                || name.contains("coretemp")
                || name.contains("k10temp")
                || name.contains("zenpower")
        })
        .map(|sensor| sensor.temperature_c);

    SensorInfo {
        cpu_temperature_c,
        temperatures,
    }
}

fn read_thermal_zones(root: &Path) -> Vec<TemperatureSensor> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut temperatures = Vec::new();

    for entry in entries.flatten() {
        let dir = entry.path();

        let Some(file_name) = dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !file_name.starts_with("thermal_zone") {
            continue;
        }

        let Ok(raw) = fs::read_to_string(dir.join("temp")) else {
            continue;
        };

        let Ok(value) = raw.trim().parse::<f64>() else {
            continue;
        };

        let temperature_c = if value.abs() > 1000.0 {
            value / 1000.0
        } else {
            value
        };

        if !(-20.0..=150.0).contains(&temperature_c) {
            continue;
        }

        let name = fs::read_to_string(dir.join("type"))
            .map(|value| value.trim().to_string())
            .unwrap_or_else(|_| file_name.to_string());

        temperatures.push(TemperatureSensor {
            name,
            temperature_c,
        });
    }

    temperatures
}

fn read_hwmon_temperatures() -> Vec<TemperatureSensor> {
    let root = Path::new("/sys/class/hwmon");
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut temperatures = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        let chip = fs::read_to_string(dir.join("name"))
            .map(|value| value.trim().to_owned())
            .unwrap_or_else(|_| "hwmon".into());

        let Ok(inputs) = fs::read_dir(&dir) else {
            continue;
        };
        for input in inputs.flatten() {
            let name = input.file_name().to_string_lossy().to_string();
            if !name.starts_with("temp") || !name.ends_with("_input") {
                continue;
            }

            let Ok(raw) = fs::read_to_string(input.path()) else {
                continue;
            };
            let Ok(value) = raw.trim().parse::<f64>() else {
                continue;
            };
            let temperature_c = if value.abs() > 1000.0 {
                value / 1000.0
            } else {
                value
            };
            if temperature_c > 0.0 && (-20.0..=150.0).contains(&temperature_c) {
                let index = name.trim_start_matches("temp").trim_end_matches("_input");
                temperatures.push(TemperatureSensor {
                    name: format!("{chip} sensor {index}"),
                    temperature_c,
                });
            }
        }
    }

    temperatures
}
