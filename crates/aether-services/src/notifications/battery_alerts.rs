
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::{task::JoinHandle, time::sleep};
use zbus::{Connection, Proxy};
use zvariant::OwnedValue;

const LOW_THRESHOLDS: [u8; 6] = [20, 15, 10, 5, 3, 2];
const HIGH_THRESHOLDS: [u8; 4] = [80, 90, 95, 100];

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const DBUS_RETRY_INTERVAL: Duration = Duration::from_secs(5);

const BUS_NAME: &str = "org.freedesktop.Notifications";
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";
const INTERFACE: &str = "org.freedesktop.Notifications";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatteryMode {
    Charging,
    Discharging,
    Other,
}

#[derive(Debug, Clone, Copy)]
struct BatteryReading {
    level: u8,
    mode: BatteryMode,
}

pub fn start_battery_alerts() -> JoinHandle<()> {
    tokio::spawn(async {
        if let Err(error) = monitor_battery().await {
            eprintln!("Aether battery monitor stopped: {error}");
        }
    })
}

async fn monitor_battery() -> io::Result<()> {
    let battery_path = find_battery()?;

    let connection = loop {
        match Connection::session().await {
            Ok(connection) => break connection,
            Err(error) => {
                eprintln!("Battery alerts: session D-Bus unavailable: {error}");
                sleep(DBUS_RETRY_INTERVAL).await;
            }
        }
    };

    let mut previous: Option<BatteryReading> = None;
    let mut alerted_low = HashSet::<u8>::new();
    let mut alerted_high = HashSet::<u8>::new();

    loop {
        match read_battery(&battery_path) {
            Ok(current) => {
                if let Some(old) = previous {
                    // Notify only on a genuine charging/discharging transition.
                    if old.mode != current.mode {
                        match current.mode {
                            BatteryMode::Charging => {
                                notify_retry(
                                    &connection,
                                    "Charger connected",
                                    &format!(
                                        "Battery is charging at {}%.",
                                        current.level
                                    ),
                                    "battery-good-charging",
                                )
                                .await;
                            }
                            BatteryMode::Discharging => {
                                notify_retry(
                                    &connection,
                                    "Charger disconnected",
                                    &format!(
                                        "Running on battery at {}%.",
                                        current.level
                                    ),
                                    "battery-good",
                                )
                                .await;
                            }
                            BatteryMode::Other => {}
                        }

                        // A new charging/discharging cycle gets a fresh set
                        // of threshold alerts.
                        alerted_low.clear();
                        alerted_high.clear();
                    }

                    match current.mode {
                        BatteryMode::Discharging => {
                            for threshold in LOW_THRESHOLDS {
                                let crossed =
                                    old.level > threshold
                                        && current.level <= threshold;

                                // Also handle the first observation below a
                                // threshold, including after a status change.
                                let already_below =
                                    current.level <= threshold
                                        && !alerted_low.contains(&threshold)
                                        && (old.mode != current.mode
                                            || crossed);

                                if crossed || already_below {
                                    if notify_retry(
                                        &connection,
                                        "Low battery",
                                        &format!(
                                            "Battery is at {}%. Connect your charger.",
                                            current.level
                                        ),
                                        "battery-caution",
                                    )
                                    .await
                                    {
                                        alerted_low.insert(threshold);
                                    }
                                }
                            }
                        }

                        BatteryMode::Charging => {
                            for threshold in HIGH_THRESHOLDS {
                                let crossed =
                                    old.level < threshold
                                        && current.level >= threshold;

                                let already_above =
                                    current.level >= threshold
                                        && !alerted_high.contains(&threshold)
                                        && old.mode != current.mode;

                                if crossed || already_above {
                                    let (title, message, icon) =
                                        if threshold == 100 {
                                            (
                                                "Battery fully charged",
                                                "Battery reached 100%.".to_string(),
                                                "battery-full",
                                            )
                                        } else {
                                            (
                                                "Battery charging",
                                                format!(
                                                    "Battery is at {}%.",
                                                    current.level
                                                ),
                                                "battery-full",
                                            )
                                        };

                                    if notify_retry(
                                        &connection,
                                        title,
                                        &message,
                                        icon,
                                    )
                                    .await
                                    {
                                        alerted_high.insert(threshold);
                                    }
                                }
                            }
                        }

                        BatteryMode::Other => {}
                    }
                } else {
                    // First observation: don't invent a plug/unplug event.
                    // But alert if starting with critically low battery.
                    if current.mode == BatteryMode::Discharging {
                        for threshold in LOW_THRESHOLDS {
                            if current.level <= threshold
                                && notify_retry(
                                    &connection,
                                    "Low battery",
                                    &format!(
                                        "Battery is at {}%. Connect your charger.",
                                        current.level
                                    ),
                                    "battery-caution",
                                )
                                .await
                            {
                                alerted_low.insert(threshold);
                            }
                        }
                    }
                }

                previous = Some(current);
            }
            Err(error) => {
                eprintln!("Could not read battery state: {error}");
            }
        }

        sleep(POLL_INTERVAL).await;
    }
}

async fn notify_retry(
    connection: &Connection,
    title: &str,
    message: &str,
    icon: &str,
) -> bool {
    match send_notification(connection, title, message, icon).await {
        Ok(()) => true,
        Err(error) => {
            eprintln!("Could not send battery notification: {error}");
            false
        }
    }
}

fn find_battery() -> io::Result<PathBuf> {
    for entry in fs::read_dir("/sys/class/power_supply")? {
        let entry = entry?;
        let path = entry.path();

        let Ok(battery_type) = fs::read_to_string(path.join("type")) else {
            continue;
        };

        if battery_type.trim() != "Battery" {
            continue;
        }

        if path.join("capacity").is_file() && path.join("status").is_file() {
            return Ok(path);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No battery found in /sys/class/power_supply",
    ))
}

fn read_battery(path: &Path) -> io::Result<BatteryReading> {
    let level = fs::read_to_string(path.join("capacity"))?
        .trim()
        .parse::<u8>()
        .map_err(io::Error::other)?;

    let status = fs::read_to_string(path.join("status"))?;

    let mode = match status.trim() {
        "Charging" => BatteryMode::Charging,
        "Discharging" => BatteryMode::Discharging,
        _ => BatteryMode::Other,
    };

    Ok(BatteryReading {
        level: level.min(100),
        mode,
    })
}

async fn send_notification(
    connection: &Connection,
    summary: &str,
    body: &str,
    icon: &str,
) -> zbus::Result<()> {
    let proxy = Proxy::new(
        connection,
        BUS_NAME,
        OBJECT_PATH,
        INTERFACE,
    )
    .await?;

    let actions: Vec<String> = Vec::new();
    let hints: std::collections::HashMap<String, OwnedValue> =
        std::collections::HashMap::new();

    let args = (
        "Aether",
        0_u32,
        icon,
        summary,
        body,
        actions,
        hints,
        5000_i32,
    );

    let _: u32 = proxy.call("Notify", &args).await?;
    Ok(())
}
