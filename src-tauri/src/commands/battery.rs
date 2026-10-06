use crate::models::BatteryStatus;
use crate::services::BatteryService;

#[tauri::command]
pub fn battery_status() -> Result<BatteryStatus, String> {
    BatteryService::new()
        .status()
        .map_err(|error| error.to_string())
}