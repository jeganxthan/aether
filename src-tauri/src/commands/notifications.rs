
use tauri::State;

use crate::{
    models::notification::Notification,
    services::notifications::NotificationManager,
};

#[tauri::command]
pub fn notification_history(
    manager: State<'_, NotificationManager>,
    limit: Option<u32>,
) -> Result<Vec<Notification>, String> {
    manager.history(limit.unwrap_or(100))
}

#[tauri::command]
pub fn notification_mark_read(
    manager: State<'_, NotificationManager>,
    id: u32,
) -> Result<(), String> {
    manager.mark_read(id)
}

#[tauri::command]
pub fn notification_dismiss(
    manager: State<'_, NotificationManager>,
    id: u32,
) -> Result<(), String> {
    manager.dismiss(id).map(|_| ())
}

#[tauri::command]
pub fn notification_clear_history(
    manager: State<'_, NotificationManager>,
) -> Result<(), String> {
    manager.clear_history()
}
