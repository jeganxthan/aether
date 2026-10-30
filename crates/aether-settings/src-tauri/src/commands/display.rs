use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub current: bool,
    pub preferred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayOutput {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub modes: Vec<DisplayMode>,
    pub current_mode: Option<DisplayMode>,
    pub scale: f64,
    pub transform: String,
    pub x: i32,
    pub y: i32,
}

fn run_wlr_randr(args: &[String]) -> Result<String, String> {
    let output = Command::new("wlr-randr")
        .args(args)
        .output()
        .map_err(|e| format!("Could not execute wlr-randr: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "wlr-randr failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn primary_display_file() -> Result<PathBuf, String> {
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or("Could not determine the configuration directory")?;

    Ok(config_dir.join("aether-shell/primary-display"))
}

fn display_layout_file() -> Result<PathBuf, String> {
    let path = primary_display_file()?;
    Ok(path.with_file_name("display-layout"))
}

fn parse_mode(line: &str) -> Option<DisplayMode> {
    let line = line.trim();
    let (dimensions, rest) = line.split_once(" px,")?;
    let (width, height) = dimensions.split_once('x')?;

    let refresh_text = rest.trim().split_whitespace().next()?;
    let refresh_rate = refresh_text.parse::<f64>().ok()?;

    Some(DisplayMode {
        width: width.parse().ok()?,
        height: height.parse().ok()?,
        refresh_rate,
        current: line.contains("(current") || line.contains(", current"),
        preferred: line.contains("preferred"),
    })
}

fn parse_outputs(text: &str) -> Vec<DisplayOutput> {
    let mut outputs = Vec::new();
    let mut current: Option<DisplayOutput> = None;
    let mut reading_modes = false;

    for line in text.lines() {
        if !line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
            if let Some(output) = current.take() {
                outputs.push(output);
            }

            let header = line.trim();
            let name = header.split_whitespace().next().unwrap_or("").to_string();

            let description = header
                .split_once('"')
                .and_then(|(_, rest)| rest.split_once('"'))
                .map(|(desc, _)| desc.to_string())
                .unwrap_or_default();

            current = Some(DisplayOutput {
                name,
                description,
                enabled: false,
                modes: Vec::new(),
                current_mode: None,
                scale: 1.0,
                transform: "normal".into(),
                x: 0,
                y: 0,
            });

            reading_modes = false;
            continue;
        }

        let Some(output) = current.as_mut() else {
            continue;
        };

        let line = line.trim();

        if line == "Modes:" {
            reading_modes = true;
            continue;
        }

        if line.starts_with("Enabled:") {
            output.enabled = line.ends_with("yes");
            reading_modes = false;
        } else if line.starts_with("Scale:") {
            output.scale = line
                .split_once(':')
                .and_then(|(_, value)| value.trim().parse().ok())
                .unwrap_or(1.0);
            reading_modes = false;
        } else if line.starts_with("Transform:") {
            output.transform = line
                .split_once(':')
                .map(|(_, value)| value.trim().to_string())
                .unwrap_or_else(|| "normal".into());
            reading_modes = false;
        } else if line.starts_with("Position:") {
            if let Some((x, y)) = line
                .split_once(':')
                .and_then(|(_, value)| value.trim().split_once(','))
            {
                output.x = x.trim().parse().unwrap_or(0);
                output.y = y.trim().parse().unwrap_or(0);
            }
            reading_modes = false;
        } else if reading_modes {
            if let Some(mode) = parse_mode(line) {
                if mode.current {
                    output.current_mode = Some(mode.clone());
                }
                output.modes.push(mode);
            }
        }
    }

    if let Some(output) = current {
        outputs.push(output);
    }

    outputs
}

#[tauri::command]
pub fn display_list() -> Result<Vec<DisplayOutput>, String> {
    let output = run_wlr_randr(&[])?;
    Ok(parse_outputs(&output))
}

#[tauri::command]
pub fn display_primary_get() -> Result<Option<String>, String> {
    let path = primary_display_file()?;

    match std::fs::read_to_string(path) {
        Ok(name) => {
            let name = name.trim();
            Ok((!name.is_empty()).then(|| name.to_owned()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Could not read the primary display setting: {error}"
        )),
    }
}

#[tauri::command]
pub fn display_primary_set(name: String) -> Result<(), String> {
    if name.is_empty() || name.contains('/') || name.contains('\0') {
        return Err("Invalid primary display name.".into());
    }

    let outputs = display_list()?;
    if !outputs
        .iter()
        .any(|output| output.enabled && output.name == name)
    {
        return Err(format!("Enabled display '{name}' was not found."));
    }

    let path = primary_display_file()?;
    let parent = path
        .parent()
        .ok_or("Could not determine the configuration directory")?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create the display settings directory: {error}"))?;

    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, name.as_bytes())
        .map_err(|error| format!("Could not save the primary display setting: {error}"))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("Could not activate the primary display setting: {error}"))
}

#[tauri::command]
pub fn display_layout_get() -> Result<String, String> {
    let path = display_layout_file()?;
    match std::fs::read_to_string(path) {
        Ok(value) if matches!(value.trim(), "extend" | "mirror" | "external") => {
            Ok(value.trim().to_owned())
        }
        Ok(_) => Ok("extend".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok("extend".to_owned()),
        Err(error) => Err(format!("Could not read the display layout: {error}")),
    }
}

#[tauri::command]
pub fn display_layout_set(layout: String) -> Result<(), String> {
    if !matches!(layout.as_str(), "extend" | "mirror" | "external") {
        return Err("Invalid display layout.".into());
    }

    let outputs = display_list()?;
    let connected = outputs.iter().collect::<Vec<_>>();
    if connected.is_empty() {
        return Err("No displays were found.".into());
    }

    match layout.as_str() {
        "mirror" => {
            for output in connected.iter().skip(1) {
                let Some(mode) = output
                    .current_mode
                    .as_ref()
                    .or_else(|| output.modes.iter().find(|mode| mode.preferred))
                    .or_else(|| output.modes.first())
                else {
                    continue;
                };
                run_wlr_randr(&vec![
                    "--output".into(),
                    output.name.clone(),
                    "--on".into(),
                    "--mode".into(),
                    format!("{}x{}@{:.3}Hz", mode.width, mode.height, mode.refresh_rate),
                    "--pos".into(),
                    "0,0".into(),
                ])?;
            }
        }
        "external" => {
            for output in &connected {
                let lower = output.name.to_ascii_lowercase();
                if lower.starts_with("edp") || lower.starts_with("lvds") || lower.starts_with("dsi")
                {
                    run_wlr_randr(&vec![
                        "--output".into(),
                        output.name.clone(),
                        "--off".into(),
                    ])?;
                }
            }
        }
        "extend" => {
            for output in &connected {
                if output.enabled {
                    continue;
                }
                let Some(mode) = output
                    .modes
                    .iter()
                    .find(|mode| mode.preferred)
                    .or_else(|| output.modes.first())
                else {
                    continue;
                };
                run_wlr_randr(&vec![
                    "--output".into(),
                    output.name.clone(),
                    "--on".into(),
                    "--mode".into(),
                    format!("{}x{}@{:.3}Hz", mode.width, mode.height, mode.refresh_rate),
                    "--pos".into(),
                    format!("{},{}", output.x, output.y),
                ])?;
            }
        }
        _ => unreachable!(),
    }

    let path = display_layout_file()?;
    let parent = path
        .parent()
        .ok_or("Could not determine display settings directory")?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create display settings directory: {error}"))?;
    std::fs::write(path, layout).map_err(|error| format!("Could not save display layout: {error}"))
}

#[tauri::command]
pub fn display_apply(
    name: String,
    width: u32,
    height: u32,
    refresh_rate: f64,
    scale: f64,
    transform: String,
    x: i32,
    y: i32,
) -> Result<(), String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\0')
        || !scale.is_finite()
        || !(0.5..=4.0).contains(&scale)
        || !refresh_rate.is_finite()
        || refresh_rate <= 0.0
    {
        return Err("Invalid display settings.".into());
    }

    let outputs = display_list()?;
    let display = outputs
        .iter()
        .find(|output| output.name == name)
        .ok_or_else(|| format!("Display '{name}' was not found."))?;

    let supported = display.modes.iter().any(|mode| {
        mode.width == width
            && mode.height == height
            && (mode.refresh_rate - refresh_rate).abs() < 0.1
    });

    if !supported {
        return Err("The selected resolution and refresh rate are not supported.".into());
    }

    const TRANSFORMS: &[&str] = &[
        "normal",
        "90",
        "180",
        "270",
        "flipped",
        "flipped-90",
        "flipped-180",
        "flipped-270",
    ];

    if !TRANSFORMS.contains(&transform.as_str()) {
        return Err("Unsupported display orientation.".into());
    }

    let mode = format!("{width}x{height}@{refresh_rate:.3}Hz");

    let args = vec![
        "--output".into(),
        name,
        "--mode".into(),
        mode,
        "--scale".into(),
        scale.to_string(),
        "--transform".into(),
        transform,
        "--pos".into(),
        format!("{x},{y}"),
    ];

    run_wlr_randr(&args)?;
    Ok(())
}
