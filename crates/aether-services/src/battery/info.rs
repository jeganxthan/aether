use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const UPOWER_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

static UPOWER_CACHE: OnceLock<Mutex<Option<(Instant, Option<UpowerEstimate>)>>> = OnceLock::new();

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

struct BatteryCell {
    path: std::path::PathBuf,
    capacity: u8,
    status: BatteryStatus,
}

#[derive(Clone, Copy)]
struct UpowerEstimate {
    status: BatteryStatus,
    duration: Duration,
}

/// Read and combine the system batteries exposed by Linux.
pub fn battery() -> Option<Battery> {
    battery_from(Path::new("/sys/class/power_supply"), upower_estimate())
}

fn ac_online() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if read_string(&path, "type").as_deref() != Some("Mains") {
            continue;
        }
        if read_u64(&path, "online") == Some(1) {
            return true;
        }
    }

    false
}

fn battery_from(root: &Path, upower: Option<UpowerEstimate>) -> Option<Battery> {
    let mut cells: Vec<BatteryCell> = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| read_string(path, "type").as_deref() == Some("Battery"))
        .filter(|path| read_u64(path, "present") != Some(0))
        .filter_map(|path| {
            let capacity = read_u64(&path, "capacity")?.min(100) as u8;
            let status = match read_string(&path, "status").as_deref() {
                Some("Charging") => BatteryStatus::Charging,
                Some("Discharging") => BatteryStatus::Discharging,
                Some("Full") => BatteryStatus::Full,
                Some("Not charging") => BatteryStatus::NotCharging,
                _ => BatteryStatus::Unknown,
            };
            Some(BatteryCell {
                path,
                capacity,
                status,
            })
        })
        .collect();

    let system_cells = cells
        .iter()
        .filter(|cell| read_string(&cell.path, "scope").as_deref() == Some("System"))
        .count();
    if system_cells > 0 {
        cells.retain(|cell| read_string(&cell.path, "scope").as_deref() == Some("System"));
    }

    if cells.is_empty() {
        return None;
    }

    let mut status = combined_status(&cells);

    // A charger can be connected while the battery reports "Not charging"
    // (for example when a charge limit is reached). Treat that as connected
    // so the shell does not claim the machine is running on battery.
    if status == BatteryStatus::NotCharging && ac_online() {
        status = BatteryStatus::Charging;
    }

    Some(Battery {
        percentage: combined_percentage(&cells),
        status,
        remaining_time: (status == BatteryStatus::Discharging)
            .then(|| {
                upower
                    .filter(|estimate| estimate.status == status)
                    .map(|estimate| estimate.duration)
                    .or_else(|| estimate_group(&cells, false))
            })
            .flatten(),
        time_to_full: (status == BatteryStatus::Charging)
            .then(|| {
                upower
                    .filter(|estimate| estimate.status == status)
                    .map(|estimate| estimate.duration)
                    .or_else(|| estimate_group(&cells, true))
            })
            .flatten(),
    })
}

fn upower_estimate() -> Option<UpowerEstimate> {
    let cache = UPOWER_CACHE.get_or_init(|| Mutex::new(None));
    let mut cached = cache.lock().ok()?;
    if let Some((sampled_at, estimate)) = *cached
        && sampled_at.elapsed() < UPOWER_REFRESH_INTERVAL
    {
        return estimate;
    }

    let estimate = Command::new("timeout")
        .args([
            "2s",
            "upower",
            "-i",
            "/org/freedesktop/UPower/devices/DisplayDevice",
        ])
        .env("LC_ALL", "C")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| parse_upower_estimate(&String::from_utf8_lossy(&output.stdout)));

    *cached = Some((Instant::now(), estimate));
    estimate
}

fn parse_upower_estimate(output: &str) -> Option<UpowerEstimate> {
    let fields: std::collections::HashMap<_, _> = output
        .lines()
        .filter_map(|line| line.trim().split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect();

    if fields.get("power supply") != Some(&"yes") || fields.get("present") != Some(&"yes") {
        return None;
    }

    let status = match fields.get("state").copied()? {
        "charging" => BatteryStatus::Charging,
        "discharging" => BatteryStatus::Discharging,
        "fully-charged" => BatteryStatus::Full,
        _ => return None,
    };
    let key = match status {
        BatteryStatus::Discharging => "time to empty",
        BatteryStatus::Charging => "time to full",
        _ => return None,
    };
    let duration = parse_upower_duration(fields.get(key).copied()?)?;

    Some(UpowerEstimate { status, duration })
}

fn parse_upower_duration(value: &str) -> Option<Duration> {
    let mut parts = value.split_whitespace();
    let amount = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next()?.trim_end_matches('s');
    let seconds = match unit {
        "second" => amount,
        "minute" => amount * 60.0,
        "hour" => amount * 3600.0,
        "day" => amount * 86_400.0,
        _ => return None,
    };

    if !seconds.is_finite() || seconds <= 0.0 || seconds >= u64::MAX as f64 {
        return None;
    }

    Some(Duration::from_secs_f64(seconds))
}

fn combined_status(cells: &[BatteryCell]) -> BatteryStatus {
    if cells
        .iter()
        .any(|cell| cell.status == BatteryStatus::Discharging)
    {
        BatteryStatus::Discharging
    } else if cells
        .iter()
        .any(|cell| cell.status == BatteryStatus::Charging)
    {
        BatteryStatus::Charging
    } else if cells.iter().all(|cell| cell.status == BatteryStatus::Full) {
        BatteryStatus::Full
    } else if cells
        .iter()
        .any(|cell| cell.status == BatteryStatus::NotCharging)
    {
        BatteryStatus::NotCharging
    } else {
        BatteryStatus::Unknown
    }
}

fn combined_percentage(cells: &[BatteryCell]) -> u8 {
    let energy_weights: Option<Vec<_>> = cells
        .iter()
        .map(|cell| {
            let weight = read_u64(&cell.path, "energy_full")? as u128;
            (weight > 0).then_some((cell.capacity as u128, weight))
        })
        .collect();
    if let Some(percentage) = energy_weights.as_deref().and_then(weighted_percentage) {
        return percentage;
    }

    let charge_weights: Option<Vec<_>> = cells
        .iter()
        .map(|cell| {
            let weight = read_u64(&cell.path, "charge_full")? as u128;
            (weight > 0).then_some((cell.capacity as u128, weight))
        })
        .collect();
    if let Some(percentage) = charge_weights.as_deref().and_then(weighted_percentage) {
        return percentage;
    }

    (cells.iter().map(|cell| cell.capacity as u16).sum::<u16>() / cells.len() as u16) as u8
}

fn weighted_percentage(weights: &[(u128, u128)]) -> Option<u8> {
    let total_weight: u128 = weights.iter().map(|(_, weight)| weight).sum();
    if total_weight == 0 {
        return None;
    }

    let weighted_sum: u128 = weights
        .iter()
        .map(|(percent, weight)| percent * weight)
        .sum();
    Some(((weighted_sum + total_weight / 2) / total_weight).min(100) as u8)
}

fn estimate_group(cells: &[BatteryCell], charging: bool) -> Option<Duration> {
    let status = if charging {
        BatteryStatus::Charging
    } else {
        BatteryStatus::Discharging
    };
    let active: Vec<_> = cells.iter().filter(|cell| cell.status == status).collect();
    if active.is_empty() {
        return None;
    }

    let energy_pairs: Option<Vec<_>> = active
        .iter()
        .map(|cell| energy_pair(&cell.path, charging))
        .collect();
    if let Some(pairs) = energy_pairs {
        if let Some(duration) = duration_from_pairs(&pairs) {
            return Some(duration);
        }
    }

    let charge_pairs: Option<Vec<_>> = active
        .iter()
        .map(|cell| charge_pair(&cell.path, charging))
        .collect();
    charge_pairs.as_deref().and_then(duration_from_pairs)
}

fn energy_pair(path: &Path, charging: bool) -> Option<(u64, u64)> {
    let now = read_u64(path, "energy_now")?;
    let amount = if charging {
        read_u64(path, "energy_full")?.saturating_sub(now)
    } else {
        now
    };
    Some((amount, read_u64(path, "power_now")?))
}

fn charge_pair(path: &Path, charging: bool) -> Option<(u64, u64)> {
    let now = read_u64(path, "charge_now")?;
    let amount = if charging {
        read_u64(path, "charge_full")?.saturating_sub(now)
    } else {
        now
    };
    Some((amount, read_u64(path, "current_now")?))
}

fn duration_from_pairs(pairs: &[(u64, u64)]) -> Option<Duration> {
    let amount: u128 = pairs.iter().map(|(amount, _)| *amount as u128).sum();
    let rate: u128 = pairs.iter().map(|(_, rate)| *rate as u128).sum();
    if rate == 0 {
        return None;
    }

    let seconds = amount as f64 / rate as f64 * 3600.0;
    if !seconds.is_finite() || seconds >= u64::MAX as f64 {
        return None;
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(std::path::PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("aether-battery-info-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn cell(&self, name: &str, values: &[(&str, &str)]) {
            let path = self.0.join(name);
            fs::create_dir_all(&path).unwrap();
            for (key, value) in values {
                fs::write(path.join(key), value).unwrap();
            }
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn combines_battery_percentage_by_full_energy() {
        let root = TestRoot::new();
        root.cell(
            "BAT0",
            &[
                ("type", "Battery"),
                ("scope", "System"),
                ("capacity", "50"),
                ("status", "Discharging"),
                ("energy_now", "500"),
                ("energy_full", "1000"),
                ("power_now", "100"),
            ],
        );
        root.cell(
            "BAT1",
            &[
                ("type", "Battery"),
                ("scope", "System"),
                ("capacity", "25"),
                ("status", "Discharging"),
                ("energy_now", "250"),
                ("energy_full", "3000"),
                ("power_now", "300"),
            ],
        );
        root.cell(
            "MOUSE",
            &[
                ("type", "Battery"),
                ("scope", "Device"),
                ("capacity", "1"),
                ("status", "Discharging"),
            ],
        );

        let battery = battery_from(&root.0, None).unwrap();
        assert_eq!(battery.percentage, 31);
        assert_eq!(battery.remaining_time, Some(Duration::from_secs(6750)));
    }

    #[test]
    fn does_not_estimate_time_when_any_active_cell_lacks_rate_data() {
        let root = TestRoot::new();
        for (name, values) in [
            (
                "BAT0",
                &[
                    ("type", "Battery"),
                    ("capacity", "50"),
                    ("status", "Discharging"),
                    ("energy_now", "500"),
                    ("energy_full", "1000"),
                    ("power_now", "100"),
                ][..],
            ),
            (
                "BAT1",
                &[
                    ("type", "Battery"),
                    ("capacity", "50"),
                    ("status", "Discharging"),
                    ("energy_now", "500"),
                    ("energy_full", "1000"),
                ][..],
            ),
        ] {
            root.cell(name, values);
        }

        assert_eq!(battery_from(&root.0, None).unwrap().remaining_time, None);
    }

    #[test]
    fn reports_only_system_batteries_when_scope_is_available() {
        let root = TestRoot::new();
        root.cell(
            "BAT0",
            &[
                ("type", "Battery"),
                ("scope", "System"),
                ("capacity", "80"),
                ("status", "Full"),
            ],
        );
        root.cell(
            "MOUSE",
            &[
                ("type", "Battery"),
                ("scope", "Device"),
                ("capacity", "10"),
                ("status", "Discharging"),
            ],
        );

        let battery = battery_from(&root.0, None).unwrap();
        assert_eq!(battery.percentage, 80);
        assert_eq!(battery.status, BatteryStatus::Full);
    }

    #[test]
    fn prefers_matching_upower_estimate_over_instantaneous_sysfs_rate() {
        let root = TestRoot::new();
        root.cell(
            "BAT0",
            &[
                ("type", "Battery"),
                ("capacity", "50"),
                ("status", "Discharging"),
                ("energy_now", "500"),
                ("energy_full", "1000"),
                ("power_now", "100"),
            ],
        );
        let upower = UpowerEstimate {
            status: BatteryStatus::Discharging,
            duration: Duration::from_secs(1800),
        };

        assert_eq!(
            battery_from(&root.0, Some(upower)).unwrap().remaining_time,
            Some(Duration::from_secs(1800))
        );
    }

    #[test]
    fn parses_upower_estimates_and_requires_a_present_power_supply() {
        let output =
            "power supply: yes\n  present: yes\n  state: discharging\n  time to empty: 1.1 hours\n";
        assert_eq!(
            parse_upower_estimate(output).map(|estimate| estimate.duration),
            Some(Duration::from_secs(3960))
        );
        assert!(
            parse_upower_estimate(
                "power supply: no\n  present: yes\n  state: discharging\n  time to empty: 1.1 hours\n"
            )
            .is_none()
        );
    }
}
