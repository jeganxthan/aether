use std::sync::Arc;

use tauri::State;

use crate::models::media::MediaInfo;
use crate::services::mpris::MediaService;


#[tauri::command]
pub async fn media_status(
    media: State<'_, Arc<MediaService>>,
) -> Result<Option<MediaInfo>, String> {
    Ok(media.status().await)
}


#[tauri::command]
pub async fn media_play_pause(
    media: State<'_, Arc<MediaService>>,
) -> Result<(), String> {
    media.play_pause().await
}


#[tauri::command]
pub async fn media_next(
    media: State<'_, Arc<MediaService>>,
) -> Result<(), String> {
    media.next().await
}


#[tauri::command]
pub async fn media_previous(
    media: State<'_, Arc<MediaService>>,
) -> Result<(), String> {
    media.previous().await
}


#[tauri::command]
pub async fn media_seek(
    media: State<'_, Arc<MediaService>>,
    offset: i64,
) -> Result<(), String> {
    media.seek(offset).await
}

/// Reads an album-art file and returns its MIME type plus the raw bytes, so the
/// webview can build a blob URL. MPRIS players expose covers as `file://` paths,
/// which the asset protocol is not configured to serve.
#[tauri::command]
pub fn read_art(url: String) -> Result<(String, Vec<u8>), String> {
    let path = url
        .strip_prefix("file://")
        .ok_or_else(|| "art URL is not a file:// URL".to_string())?;
    let path = percent_decode(path);
    let bytes = std::fs::read(&path)
        .map_err(|err| format!("failed to read art: {err}"))?;
    let mime = mime_for_path(&path);
    Ok((mime, bytes))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn mime_for_path(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "webp" => "image/webp".to_string(),
        "gif" => "image/gif".to_string(),
        "bmp" => "image/bmp".to_string(),
        "svg" => "image/svg+xml".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}
