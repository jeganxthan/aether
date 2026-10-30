pub mod battery;
pub mod cpu;
pub mod memory;
pub mod profiles;
pub mod sensors;

pub use profiles::{PowerProfile, PowerProfileChange, PowerProfileError, PowerProfileService};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerSnapshot {
    pub battery: battery::BatteryInfo,
    pub cpu: cpu::CpuInfo,
    pub memory: memory::MemoryInfo,
    pub sensors: sensors::SensorInfo,
    pub profile: profiles::PowerProfileStatus,
}

pub fn snapshot() -> PowerSnapshot {
    let profile_service = profiles::PowerProfileService::new();

    PowerSnapshot {
        battery: battery::read_battery(),
        cpu: cpu::read_cpu(),
        memory: memory::read_memory(),
        sensors: sensors::read_sensors(),
        profile: profile_service.profile_status(),
    }
}
