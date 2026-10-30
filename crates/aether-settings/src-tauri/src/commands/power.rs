use aether_services::power::{self, PowerSnapshot, profiles};

#[tauri::command]
pub fn power_get_snapshot() -> PowerSnapshot {
    power::snapshot()
}

#[tauri::command]
pub fn power_set_profile(profile: String) -> Result<PowerSnapshot, String> {
    let service = profiles::PowerProfileService::new();

    let requested = profiles::PowerProfile::parse(&profile)
        .ok_or_else(|| format!("Invalid power profile: {profile}"))?;

    service
        .set_profile(requested)
        .map_err(|error| error.to_string())?;
    Ok(power::snapshot())
}
