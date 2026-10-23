
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battery {
    pub percentage: u8,
    pub status: BatteryStatus,

    /// Estimated time until the battery is empty.
    /// Available while discharging when the hardware exposes power/current.
    pub remaining_time: Option<Duration>,

    /// Estimated time until the battery is full.
    /// Available while charging when the hardware exposes power/current.
    pub time_to_full: Option<Duration>,
}

impl Battery {
    pub fn is_charging(&self) -> bool {
        self.status == BatteryStatus::Charging
    }

    pub fn is_discharging(&self) -> bool {
        self.status == BatteryStatus::Discharging
    }
}

/// Read the first system battery exposed by Linux.
pub fn battery() -> Option<Battery> {
    let root = Path::new("/sys/class/power_supply");

    let mut batteries: Vec<PathBuf> = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            read_string(path, "type").as_deref() == Some("Battery")
        })
        .collect();

    batteries.sort();

    let path = batteries.first()?;

    let percentage = read_u64(path, "capacity")?
        .min(100) as u8;

    let status = match read_string(path, "status").as_deref() {
        Some("Charging") => BatteryStatus::Charging,
        Some("Discharging") => BatteryStatus::Discharging,
        Some("Full") => BatteryStatus::Full,
        Some("Not charging") => BatteryStatus::NotCharging,
        _ => BatteryStatus::Unknown,
    };

    let remaining_time = if status == BatteryStatus::Discharging {
        estimate_time(path, false)
    } else {
        None
    };

    let time_to_full = if status == BatteryStatus::Charging {
        estimate_time(path, true)
    } else {
        None
    };

    Some(Battery {
        percentage,
        status,
        remaining_time,
        time_to_full,
    })
}

/// Estimate duration using energy/power first, then charge/current.
///
/// Linux sysfs energy is normally in µWh and power in µW.
/// Charge is normally in µAh and current in µA.
/// Dividing either pair gives hours.
fn estimate_time(path: &Path, charging: bool) -> Option<Duration> {
    // Prefer energy-based values when the driver exposes them.
    let energy_now = read_u64(path, "energy_now");
    let energy_full = read_u64(path, "energy_full");
    let power_now = read_u64(path, "power_now");

    if let (Some(now), Some(power)) = (energy_now, power_now) {
        let amount = if charging {
            energy_full?.saturating_sub(now)
        } else {
            now
        };

        return duration_from_units(amount, power);
    }

    // Some batteries expose charge and current instead.
    let charge_now = read_u64(path, "charge_now");
    let charge_full = read_u64(path, "charge_full");
    let current_now = read_u64(path, "current_now");

    if let (Some(now), Some(current)) = (charge_now, current_now) {
        let amount = if charging {
            charge_full?.saturating_sub(now)
        } else {
            now
        };

        return duration_from_units(amount, current);
    }

    // Some drivers expose only energy values, without a power reading.
    // In that case, an accurate live time estimate cannot be calculated.
    None
}

fn duration_from_units(amount: u64, rate: u64) -> Option<Duration> {
    if rate == 0 {
        return None;
    }

    let seconds = (amount as f64 / rate as f64) * 3600.0;

    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }

    // Avoid overflowing Duration for unusual driver values.
    let seconds = seconds.min(u64::MAX as f64);

    Some(Duration::from_secs_f64(seconds))
}

fn read_string(path: &Path, name: &str) -> Option<String> {
    fs::read_to_string(path.join(name))
        .ok()
        .map(|value| value.trim().to_string())
}

fn read_u64(path: &Path, name: &str) -> Option<u64> {
    read_string(path, name)?.parse().ok()
}

/// Format a duration for display, e.g. "2h 15m" or "45m".
pub fn format_duration(duration: Duration) -> String {
    let total_minutes = duration.as_secs() / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}
