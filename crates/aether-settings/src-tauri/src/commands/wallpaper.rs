use serde::{Deserialize, Serialize};
use std::{
    fs,
    hash::{Hash, Hasher},
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

fn wallpaper_cache_dir() -> Result<PathBuf, String> {
    let cache_dir = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or("Could not determine the wallpaper cache directory")?
        .join("aether-shell");
    fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Could not create wallpaper cache: {error}"))?;
    Ok(cache_dir)
}

fn swaybg_image(path: &Path, cache_dir: &Path) -> Result<PathBuf, String> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("webp"))
    {
        return Ok(path.to_path_buf());
    }

    let metadata =
        fs::metadata(path).map_err(|error| format!("Could not inspect WebP wallpaper: {error}"))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |time| time.as_nanos());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified.hash(&mut hasher);

    let png_path = cache_dir.join(format!("wallpaper-{:016x}.png", hasher.finish()));
    if png_path.is_file() {
        return Ok(png_path);
    }

    let decoded = image::ImageReader::open(path)
        .map_err(|error| format!("Could not open WebP wallpaper: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("Could not identify WebP wallpaper: {error}"))?
        .decode()
        .map_err(|error| format!("Could not decode WebP wallpaper: {error}"))?;

    let temporary_path = png_path.with_extension("png.tmp");
    decoded
        .save_with_format(&temporary_path, image::ImageFormat::Png)
        .map_err(|error| format!("Could not convert WebP wallpaper to PNG: {error}"))?;
    fs::rename(&temporary_path, &png_path)
        .map_err(|error| format!("Could not finish WebP wallpaper conversion: {error}"))?;

    Ok(png_path)
}

fn apply_wallpaper(state: &WallpaperState, path: &Path) -> Result<(), String> {
    let path = validate_image(path)?;
    let mut process = state
        .process
        .lock()
        .map_err(|_| "Wallpaper process lock is unavailable")?;

    let display_path = if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("webp"))
    {
        swaybg_image(&path, &wallpaper_cache_dir()?)?
    } else {
        path.clone()
    };

    if let Some(child) = process.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }

    let child = Command::new("swaybg")
        .args(["-i", display_path.to_string_lossy().as_ref(), "-m", "fill"])
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
pub fn wallpaper_restore(app: AppHandle, state: State<'_, WallpaperState>) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[test]
    fn converts_webp_wallpaper_to_png_for_swaybg() {
        let test_dir =
            std::env::temp_dir().join(format!("aether-wallpaper-webp-test-{}", std::process::id()));
        fs::create_dir_all(&test_dir).expect("create test directory");
        let webp_path = test_dir.join("source.webp");
        let source = image::DynamicImage::new_rgb8(2, 3);
        source
            .save_with_format(&webp_path, image::ImageFormat::WebP)
            .expect("write WebP fixture");

        let png_path = swaybg_image(&webp_path, &test_dir).expect("convert WebP fixture");
        let converted = image::ImageReader::open(png_path)
            .expect("open converted PNG")
            .decode()
            .expect("decode converted PNG");

        assert_eq!(converted.dimensions(), (2, 3));
        fs::remove_dir_all(test_dir).expect("remove test directory");
    }
}
