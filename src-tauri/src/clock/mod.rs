use chrono::Local;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

fn current_time() -> String {
    Local::now().format("%H:%M").to_string()
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let time = current_time();

            let _ = app.emit("clock:update", time);

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

#[tauri::command]
pub fn get_time() -> String {
    current_time()
}