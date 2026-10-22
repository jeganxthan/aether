use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{clipboard, icons, notify, region};

/*
 * grim does not write to stdout on its own: with no file name it drops a
 * timestamped png into $GRIM_DEFAULT_DIR, or into the current directory when
 * that is unset, and only writes to stdout when the file name is "-". So it
 * always gets a path here and the bytes are read back from that file.
 */

// where the pictures live: $XDG_PICTURES_DIR, then $HOME/Pictures
fn pictures_dir() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("XDG_PICTURES_DIR") {
        let dir = PathBuf::from(dir);

        // the spec says a relative path is to be ignored
        if dir.is_absolute() {
            return Ok(dir);
        }
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("HOME directory is not set")?;

    Ok(home.join("Pictures"))
}

// the folder the screenshots are kept in
fn screenshot_dir() -> Result<PathBuf, String> {
    let directory = pictures_dir()?.join("Screenshots");

    fs::create_dir_all(&directory)
        .map_err(|e| format!("Failed to create screenshot directory: {e}"))?;

    Ok(directory)
}

/*
 * A name that reads well and never lands on an older shot: the time of day
 * first, then a counter for the shots taken inside the same second.
 */
fn screenshot_path() -> Result<PathBuf, String> {
    let directory = screenshot_dir()?;

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let stamp = match chrono::DateTime::from_timestamp(seconds as i64, 0) {
        Some(time) => time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d_%H-%M-%S")
            .to_string(),
        None => seconds.to_string(),
    };

    let mut path = directory.join(format!("Screenshot_{stamp}.png"));

    let mut count = 1;
    while path.exists() {
        path = directory.join(format!("Screenshot_{stamp}_{count}.png"));
        count += 1;
    }

    Ok(path)
}

// a scratch file next to the screenshots, so it can be removed right after
fn scratch_path() -> Result<PathBuf, String> {
    let directory = screenshot_dir()?;

    let scratch = directory.join(format!(".capture-{}.png", std::process::id()));

    Ok(scratch)
}

fn capture(args: &[&str], path: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("grim")
        .args(args)
        .arg(path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "grim is not installed".to_string()
            } else {
                format!("Failed to start grim: {e}")
            }
        })?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();

        return Err(if message.is_empty() {
            "Screenshot capture failed".to_string()
        } else {
            format!("Screenshot capture failed: {message}")
        });
    }

    let bytes = fs::read(path).map_err(|e| format!("Failed to read the capture: {e}"))?;

    if bytes.is_empty() {
        return Err("Screenshot capture produced an empty image".into());
    }

    Ok(bytes)
}

// captures into a scratch file and hands back the bytes
fn capture_bytes(args: &[&str]) -> Result<Vec<u8>, String> {
    let scratch = scratch_path()?;

    let result = capture(args, &scratch);

    // the scratch file is of no use either way
    let _ = fs::remove_file(&scratch);

    result
}

/// Capture the full screen. Shortcut: Super+S.
pub fn full_screenshot() -> Result<PathBuf, String> {
    let path = screenshot_path()?;
    let png = capture(&[], &path)?;

    clipboard::copy_image(&png)?;
    notify::notify(
        "Screenshot",
        &icons::icon("screenshot"),
        "Screenshot saved",
        &path.display().to_string(),
    );

    Ok(path)
}

/// Capture a specific region passed as "x,y wxh".
pub fn capture_region(geometry: &str) -> Result<Vec<u8>, String> {
    capture_bytes(&["-g", geometry])
}

/// Capture the single pixel at "x,y".
pub fn capture_point(x: i32, y: i32) -> Result<Vec<u8>, String> {
    let geometry = format!("{x},{y} 1x1");

    capture_bytes(&["-g", &geometry])
}

/// Capture a selected region. Shortcut: Super+Shift+S.
pub fn selective_screenshot() -> Result<PathBuf, String> {
    let selected = region::select_region()?;
    let png = capture_region(&selected)?;

    let path = screenshot_path()?;
    fs::write(&path, &png).map_err(|e| format!("Failed to save screenshot: {e}"))?;

    clipboard::copy_image(&png)?;
    notify::notify(
        "Screenshot",
        &icons::icon("screenshot"),
        "Screenshot saved",
        &path.display().to_string(),
    );

    Ok(path)
}
