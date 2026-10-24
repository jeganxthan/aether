use std::{io, time::Duration};

use tokio::sync::watch;

use super::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryInfo {
    pub percentage: u8,
    pub status: BatteryStatus,
}

impl BatteryInfo {
    pub fn is_charging(&self) -> bool {
        self.status == BatteryStatus::Charging
    }
}

pub struct BatteryService;

impl BatteryService {
    pub fn new() -> io::Result<Self> {
        read_battery_info().map(|_| Self).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "No readable system battery found in /sys/class/power_supply",
            )
        })
    }

    /// Read the aggregate percentage and charging status of system batteries.
    pub fn current(&self) -> io::Result<BatteryInfo> {
        read_battery_info().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "No readable system battery found in /sys/class/power_supply",
            )
        })
    }

    /// Subscribe to battery changes, polling every five seconds.
    pub fn subscribe(&self) -> watch::Receiver<Option<BatteryInfo>> {
        let (sender, receiver) = watch::channel(read_battery_info());

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                let result = read_battery_info();
                if *sender.borrow() != result {
                    sender.send_replace(result);
                }
            }
        });

        receiver
    }
}

fn read_battery_info() -> Option<BatteryInfo> {
    let battery = info::battery()?;
    let status = match battery.status {
        info::BatteryStatus::Charging => BatteryStatus::Charging,
        info::BatteryStatus::Discharging => BatteryStatus::Discharging,
        info::BatteryStatus::Full => BatteryStatus::Full,
        info::BatteryStatus::NotCharging => BatteryStatus::NotCharging,
        info::BatteryStatus::Unknown => BatteryStatus::Unknown,
    };

    Some(BatteryInfo {
        percentage: battery.percentage,
        status,
    })
}
