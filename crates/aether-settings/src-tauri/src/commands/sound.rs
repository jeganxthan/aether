use aether_services::osd::audio::{
    AudioDevices, AudioService, AudioState, MicrophoneService, MicrophoneState,
};

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

// ─── Device listing and selection ────────────────────────────

#[tauri::command]
pub fn sound_list_devices() -> Result<AudioDevices, String> {
    AudioService::devices().map_err(audio_error)
}

#[tauri::command]
pub fn sound_select_output(name: String) -> Result<AudioDevices, String> {
    AudioService::select_output(&name).map_err(audio_error)
}

#[tauri::command]
pub fn sound_select_input(name: String) -> Result<AudioDevices, String> {
    AudioService::select_input(&name).map_err(audio_error)
}

// ─── Microphone controls ─────────────────────────────────────

#[tauri::command]
pub fn sound_get_microphone_state() -> Result<MicrophoneState, String> {
    MicrophoneService::current().map_err(audio_error)
}

#[tauri::command]
pub fn sound_toggle_microphone_mute() -> Result<MicrophoneState, String> {
    MicrophoneService::toggle_mute().map_err(audio_error)
}

#[tauri::command]
pub fn sound_set_microphone_mute(muted: bool) -> Result<MicrophoneState, String> {
    MicrophoneService::set_mute(muted).map_err(audio_error)
}
