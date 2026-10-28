use aether_services::osd::audio::{AudioService, AudioState};

fn audio_error(error: std::io::Error) -> String {
    error.to_string()
}

#[tauri::command]
pub fn sound_get_state() -> Result<AudioState, String> {
    AudioService::current().map_err(audio_error)
}

#[tauri::command]
pub fn sound_set_volume(percent: u8) -> Result<AudioState, String> {
    AudioService::set_volume(percent).map_err(audio_error)
}

#[tauri::command]
pub fn sound_toggle_mute() -> Result<AudioState, String> {
    AudioService::toggle_mute().map_err(audio_error)
}

#[tauri::command]
pub fn sound_increase_volume() -> Result<AudioState, String> {
    AudioService::increase().map_err(audio_error)
}

#[tauri::command]
pub fn sound_decrease_volume() -> Result<AudioState, String> {
    AudioService::decrease().map_err(audio_error)
}