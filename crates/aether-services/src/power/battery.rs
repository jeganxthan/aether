use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub available: bool,
    pub percentage: Option<f64>,
    pub status: Option<String>,
    pub energy_now_wh: Option<f64>,
    pub energy_full_wh: Option<f64>,
    pub energy_design_wh: Option<f64>,
    pub power_w: Option<f64>,
    pub voltage_v: Option<f64>,
    pub current_a: Option<f64>,
    pub cycle_count: Option<u64>,
    pub health_percent: Option<f64>,
    pub time_remaining_minutes: Option<u64>,
}

fn read_number(path: &Path) -> Option<f64> {
    fs::read_to_string(path).ok()?.trim().parse::<f64>().ok()
}

fn read_text(path: &Path) -> Option<String> {
    Some(fs::read_to_string(path).ok()?.trim().to_string())
}

fn first_number(dir: &Path, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|name| read_number(&dir.join(name)))
}

fn micro_to_unit(value: Option<f64>) -> Option<f64> {
    value.map(|value| value / 1_000_000.0)
}

pub fn read_battery() -> BatteryInfo {
    let root = Path::new("/sys/class/power_supply");

    let Ok(entries) = fs::read_dir(root) else {
        return BatteryInfo::default();
    };

    for entry in entries.flatten() {
        let dir = entry.path();

        if read_text(&dir.join("type")).as_deref() != Some("Battery") {
            continue;
        }

        let energy_now = first_number(&dir, &["energy_now"]);
        let energy_full = first_number(&dir, &["energy_full"]);
        let energy_design = first_number(&dir, &["energy_full_design"]);

        // Linux commonly exposes energy in µWh and charge in µAh.
        // Convert charge to energy only when voltage is available.
        let voltage_micro = first_number(&dir, &["voltage_now"]);
        let charge_now = first_number(&dir, &["charge_now"]);
        let charge_full = first_number(&dir, &["charge_full"]);
        let charge_design = first_number(&dir, &["charge_full_design"]);

        let voltage_v = micro_to_unit(voltage_micro);
        let energy_now_wh =
            micro_to_unit(energy_now).or_else(|| Some(charge_now? * voltage_v? / 1_000_000.0));
        let energy_full_wh =
            micro_to_unit(energy_full).or_else(|| Some(charge_full? * voltage_v? / 1_000_000.0));
        let energy_design_wh = micro_to_unit(energy_design)
            .or_else(|| Some(charge_design? * voltage_v? / 1_000_000.0));

        let percentage = read_number(&dir.join("capacity"))
            .or_else(|| Some(energy_now? / energy_full? * 100.0))
            .map(|value| value.clamp(0.0, 100.0));

        let status = read_text(&dir.join("status"));
        let power_w = first_number(&dir, &["power_now"])
            .map(|value| value / 1_000_000.0)
            .or_else(|| {
                let current = first_number(&dir, &["current_now"])?;
                Some(current / 1_000_000.0 * voltage_v?)
            });

        let current_a = first_number(&dir, &["current_now"])
            .map(|value| value / 1_000_000.0)
            .or_else(|| Some(power_w? / voltage_v?));

        let time_remaining_minutes =
            match (status.as_deref(), energy_now_wh, energy_full_wh, power_w) {
                (Some("Discharging"), Some(now), _, Some(power)) if power > 0.05 => {
                    Some((now / power * 60.0).round() as u64)
                }
                (Some("Charging"), Some(now), Some(full), Some(power))
                    if power > 0.05 && full > now =>
                {
                    Some(((full - now) / power * 60.0).round() as u64)
                }
                _ => None,
            };

        let health_percent = match (energy_full_wh, energy_design_wh) {
            (Some(full), Some(design)) if design > 0.0 => {
                Some((full / design * 100.0).clamp(0.0, 100.0))
            }
            _ => None,
        };

        return BatteryInfo {
            available: true,
            percentage,
            status,
            energy_now_wh,
            energy_full_wh,
            energy_design_wh,
            power_w,
            voltage_v,
            current_a,
            cycle_count: read_text(&dir.join("cycle_count")).and_then(|value| value.parse().ok()),
            health_percent,
            time_remaining_minutes,
        };
    }

    BatteryInfo::default()
}
