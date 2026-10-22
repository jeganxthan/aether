
use std::{
    io,
    path::PathBuf,
    time::Duration,
};

use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

impl BatteryStatus {
    fn from_sysfs(value: &str) -> Self {
        match value.trim() {
            "Charging" => Self::Charging,
            "Discharging" => Self::Discharging,
            "Full" => Self::Full,
            "Not charging" => Self::NotCharging,
            _ => Self::Unknown,
        }
    }

    pub fn is_charging(self) -> bool {
        self == Self::Charging
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryInfo {
    pub percentage: u8,
    pub status: BatteryStatus,
}

impl BatteryInfo {
    pub fn is_charging(&self) -> bool {
        self.status.is_charging()
    }
}

pub struct BatteryService {
    battery_path: PathBuf,
}

impl BatteryService {
    pub fn new() -> io::Result<Self> {
        let root = PathBuf::from("/sys/class/power_supply");

        for entry in std::fs::read_dir(root)? {
            let path = entry?.path();

            let Ok(kind) = std::fs::read_to_string(path.join("type")) else {
                continue;
            };

            if kind.trim() == "Battery" {
                return Ok(Self { battery_path: path });
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No battery found in /sys/class/power_supply",
        ))
    }

    /// Read the current percentage and charging status.
    pub fn current(&self) -> io::Result<BatteryInfo> {
        let percentage_text =
            std::fs::read_to_string(self.battery_path.join("capacity"))?;

        let percentage: u8 = percentage_text.trim().parse().map_err(|error| {
            io::Error::new(io::ErrorKind::InvalidData, error)
        })?;

        if percentage > 100 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Battery percentage exceeds 100",
            ));
        }

        let status_text =
            std::fs::read_to_string(self.battery_path.join("status"))?;

        Ok(BatteryInfo {
            percentage,
            status: BatteryStatus::from_sysfs(&status_text),
        })
    }

    /// Subscribe to battery changes, polling every five seconds.
    pub fn subscribe(&self) -> watch::Receiver<Option<BatteryInfo>> {
        let (sender, receiver) = watch::channel(self.current().ok());
        let battery_path = self.battery_path.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                let result = read_battery_info(&battery_path);

                if *sender.borrow() != result {
                    sender.send_replace(result);
                }
            }
        });

        receiver
    }
}

fn read_battery_info(path: &PathBuf) -> Option<BatteryInfo> {
    let percentage = std::fs::read_to_string(path.join("capacity"))
        .ok()?
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|value| *value <= 100)?;

    let status = std::fs::read_to_string(path.join("status")).ok()?;

    Some(BatteryInfo {
        percentage,
        status: BatteryStatus::from_sysfs(&status),
    })
}
