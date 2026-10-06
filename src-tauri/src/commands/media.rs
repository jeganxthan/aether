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