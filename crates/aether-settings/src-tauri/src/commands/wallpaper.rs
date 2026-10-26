use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State};

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "avif"];

pub struct WallpaperState {
    process: Mutex<Option<Child>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Wallpaper {
    pub path: String,
    pub name: String,
}

fn wallpaper_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("Could not determine the home directory")?;

    Ok(home.join("Pictures").join("Wallpapers"))
}

fn settings_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("wallpaper.txt"))
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn validate_image(path: &Path) -> Result<PathBuf, String> {
    let path = path
        .canonicalize()
        .map_err(|e| format!("Cannot access image: {e}"))?;

    if !path.is_file() || !is_image(&path) {
        return Err("Choose a supported image file (JPG, PNG, WebP, BMP or AVIF).".into());
    }

    Ok(path)
}

fn apply_wallpaper(
    state: &WallpaperState,
    path: &Path,
) -> Result<(), String> {
    let path = validate_image(path)?;

    let mut process = state
        .process
        .lock()
        .map_err(|_| "Wallpaper process lock is unavailable")?;

    if let Some(child) = process.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }

    let child = Command::new("swaybg")
        .args(["-i", path.to_string_lossy().as_ref(), "-m", "fill"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start swaybg: {e}. Is swaybg installed?"))?;

    *process = Some(child);
    Ok(())
}

#[tauri::command]
pub fn wallpaper_list() -> Result<Vec<Wallpaper>, String> {
    let dir = wallpaper_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut images = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_image(path))
        .filter_map(|path| {
            let path = path.canonicalize().ok()?;
            let name = path.file_name()?.to_string_lossy().into_owned();

            Some(Wallpaper {
                path: path.to_string_lossy().into_owned(),
                name,
            })
        })
        .collect::<Vec<_>>();

    images.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(images)
}

#[tauri::command]
pub fn wallpaper_current(app: AppHandle) -> Result<Option<String>, String> {
    let file = settings_file(&app)?;

    if !file.exists() {
        return Ok(None);
    }

    let path = fs::read_to_string(file).map_err(|e| e.to_string())?;
    let path = path.trim();

    if path.is_empty() {
        return Ok(None);
    }

    Ok(Some(path.to_owned()))
}

#[tauri::command]
pub fn wallpaper_set(
    app: AppHandle,
    state: State<'_, WallpaperState>,
    path: String,
) -> Result<(), String> {
    let image = validate_image(Path::new(&path))?;

    apply_wallpaper(&state, &image)?;

    let file = settings_file(&app)?;
    fs::write(file, image.to_string_lossy().as_bytes())
        .map_err(|e| format!("Wallpaper applied, but saving the preference failed: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn wallpaper_folder() -> Result<String, String> {
    let dir = wallpaper_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn wallpaper_restore(
    app: AppHandle,
    state: State<'_, WallpaperState>,
) -> Result<(), String> {
    let Some(path) = wallpaper_current(app)? else {
        return Ok(());
    };

    apply_wallpaper(&state, Path::new(&path))
}

impl WallpaperState {
    pub fn new() -> Self {
        Self {
            process: Mutex::new(None),
        }
    }
}