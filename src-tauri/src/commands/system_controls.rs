use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeState {
    pub percent: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrightnessState {
    pub percent: u8,
}

fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {program}: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid output from {program}: {e}"))
}

fn get_volume_state() -> Result<VolumeState, String> {
    let output = run_command("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;

    let percent = output
        .split_whitespace()
        .nth(1)
        .ok_or("Could not read volume")?
        .parse::<f32>()
        .map_err(|e| format!("Invalid volume: {e}"))?;

    let muted = output.contains("[MUTED]");

    Ok(VolumeState {
        percent: (percent * 100.0).round().clamp(0.0, 100.0) as u8,
        muted,
    })
}

#[tauri::command]
pub fn volume_get() -> Result<VolumeState, String> {
    get_volume_state()
}

#[tauri::command]
pub fn volume_increase() -> Result<VolumeState, String> {
    run_command(
        "wpctl",
        &["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", "5%+"],
    )?;

    get_volume_state()
}

#[tauri::command]
pub fn volume_decrease() -> Result<VolumeState, String> {
    run_command(
        "wpctl",
        &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"],
    )?;

    get_volume_state()
}

#[tauri::command]
pub fn volume_toggle_mute() -> Result<VolumeState, String> {
    run_command(
        "wpctl",
        &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"],
    )?;

    get_volume_state()
}

fn get_brightness_state() -> Result<BrightnessState, String> {
    let current = run_command("brightnessctl", &["get"])?;
    let maximum = run_command("brightnessctl", &["max"])?;

    let current = current
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Invalid brightness: {e}"))?;

    let maximum = maximum
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Invalid maximum brightness: {e}"))?;

    if maximum == 0 {
        return Err("Brightness device reports a maximum of zero".into());
    }

    Ok(BrightnessState {
        percent: ((current as f64 / maximum as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u8,
    })
}

#[tauri::command]
pub fn brightness_get() -> Result<BrightnessState, String> {
    get_brightness_state()
}

#[tauri::command]
pub fn brightness_increase() -> Result<BrightnessState, String> {
    run_command("brightnessctl", &["set", "5%+"])?;
    get_brightness_state()
}

#[tauri::command]
pub fn brightness_decrease() -> Result<BrightnessState, String> {
    run_command("brightnessctl", &["set", "5%-"])?;
    get_brightness_state()
}