
use crate::models::ClipboardEntry;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

const HISTORY_TTL_SECONDS: i64 = 24 * 60 * 60;
const MAX_ENTRIES: usize = 500;
const MAX_TEXT_BYTES: usize = 1024 * 1024;

pub struct ClipboardState {
    pub entries: Mutex<Vec<ClipboardEntry>>,
    pub file_path: PathBuf,
}

impl ClipboardState {
    pub fn new(file_path: PathBuf) -> Self {
        let entries = fs::read_to_string(&file_path)
            .ok()
            .and_then(|data| serde_json::from_str::<Vec<ClipboardEntry>>(&data).ok())
            .unwrap_or_default();

        let state = Self {
            entries: Mutex::new(entries),
            file_path,
        };

        if let Ok(mut entries) = state.entries.lock() {
            cleanup_entries(&mut entries, now());
            trim_entries(&mut entries);
            let _ = save_entries(&state.file_path, &entries);
        }

        state
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn cleanup_entries(entries: &mut Vec<ClipboardEntry>, timestamp: i64) {
    entries.retain(|entry| {
        entry.pinned || timestamp.saturating_sub(entry.copied_at) < HISTORY_TTL_SECONDS
    });
}

fn trim_entries(entries: &mut Vec<ClipboardEntry>) {
    // Keep pinned entries, and retain the newest unpinned entries within the limit.
    let pinned_count = entries.iter().filter(|entry| entry.pinned).count();
    let unpinned_limit = MAX_ENTRIES.saturating_sub(pinned_count);

    let mut unpinned_seen = 0;
    entries.retain(|entry| {
        if entry.pinned {
            true
        } else {
            unpinned_seen += 1;
            unpinned_seen <= unpinned_limit
        }
    });
}

fn save_entries(
    file_path: &PathBuf,
    entries: &[ClipboardEntry],
) -> Result<(), String> {
    let parent = file_path
        .parent()
        .ok_or_else(|| "Invalid clipboard history path".to_string())?;

    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let temp_path = file_path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(entries).map_err(|e| e.to_string())?;

    fs::write(&temp_path, data).map_err(|e| e.to_string())?;
    fs::rename(&temp_path, file_path).map_err(|e| e.to_string())?;

    Ok(())
}

fn persist(state: &ClipboardState, entries: &[ClipboardEntry]) -> Result<(), String> {
    save_entries(&state.file_path, entries)
}

fn make_id() -> String {
    format!("clip-{}-{}", now(), std::process::id())
}

fn add_text(state: &ClipboardState, content: String) -> Result<(), String> {
    if content.trim().is_empty() || content.len() > MAX_TEXT_BYTES {
        return Ok(());
    }

    let timestamp = now();
    let mut entries = state
        .entries
        .lock()
        .map_err(|_| "Clipboard history lock failed".to_string())?;

    cleanup_entries(&mut entries, timestamp);

    if let Some(index) = entries.iter().position(|entry| entry.content == content) {
        let mut entry = entries.remove(index);
        entry.copied_at = timestamp;
        entries.insert(0, entry);
    } else {
        entries.insert(
            0,
            ClipboardEntry {
                id: make_id(),
                content,
                content_type: "text".to_string(),
                copied_at: timestamp,
                pinned: false,
            },
        );
    }

    trim_entries(&mut entries);
    persist(state, &entries)
}

fn read_system_clipboard() -> Result<String, String> {
    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", "text"])
        .output()
        .map_err(|e| format!("Could not run wl-paste. Install wl-clipboard: {e}"))?;

    if !output.status.success() {
        return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn write_system_clipboard(content: &str) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run wl-copy. Install wl-clipboard: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let status = child.wait().map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("wl-copy failed to update the clipboard".to_string());
    }

    Ok(())
}

fn emit_update(app: &AppHandle, entries: &[ClipboardEntry]) {
    let _ = app.emit("clipboard:update", entries);
}

pub fn start_clipboard_watcher(app: AppHandle) {
    thread::spawn(move || {
        let mut previous_content = String::new();

        loop {
            thread::sleep(Duration::from_millis(800));

            let content = match read_system_clipboard() {
                Ok(content) => content,
                Err(_) => continue,
            };

            if content.is_empty() || content == previous_content {
                continue;
            }

            previous_content = content.clone();

            let state = app.state::<ClipboardState>();

            if add_text(&state, content).is_ok() {
                if let Ok(entries) = state.entries.lock() {
                    emit_update(&app, &entries);
                }
            }
        }
    });
}

#[tauri::command]
pub fn clipboard_history(
    app: AppHandle,
    state: State<'_, ClipboardState>,
) -> Result<Vec<ClipboardEntry>, String> {
    let mut entries = state
        .entries
        .lock()
        .map_err(|_| "Clipboard history lock failed".to_string())?;

    cleanup_entries(&mut entries, now());
    trim_entries(&mut entries);
    persist(&state, &entries)?;

    let result = entries.clone();
    emit_update(&app, &result);

    Ok(result)
}

#[tauri::command]
pub fn clipboard_pin(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    id: String,
    pinned: bool,
) -> Result<Vec<ClipboardEntry>, String> {
    let mut entries = state
        .entries
        .lock()
        .map_err(|_| "Clipboard history lock failed".to_string())?;

    let entry = entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "Clipboard entry not found".to_string())?;

    entry.pinned = pinned;

    // Unpinning an old entry allows it to expire immediately.
    cleanup_entries(&mut entries, now());
    persist(&state, &entries)?;

    let result = entries.clone();
    emit_update(&app, &result);

    Ok(result)
}

#[tauri::command]
pub fn clipboard_delete(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    id: String,
) -> Result<Vec<ClipboardEntry>, String> {
    let mut entries = state
        .entries
        .lock()
        .map_err(|_| "Clipboard history lock failed".to_string())?;

    entries.retain(|entry| entry.id != id);
    persist(&state, &entries)?;

    let result = entries.clone();
    emit_update(&app, &result);

    Ok(result)
}

#[tauri::command]
pub fn clipboard_clear(
    app: AppHandle,
    state: State<'_, ClipboardState>,
) -> Result<Vec<ClipboardEntry>, String> {
    let mut entries = state
        .entries
        .lock()
        .map_err(|_| "Clipboard history lock failed".to_string())?;

    entries.retain(|entry| entry.pinned);
    persist(&state, &entries)?;

    let result = entries.clone();
    emit_update(&app, &result);

    Ok(result)
}

#[tauri::command]
pub fn clipboard_copy(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    id: String,
) -> Result<(), String> {
    let content = {
        let entries = state
            .entries
            .lock()
            .map_err(|_| "Clipboard history lock failed".to_string())?;

        entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.content.clone())
            .ok_or_else(|| "Clipboard entry not found".to_string())?
    };

    write_system_clipboard(&content)?;

    // wl-paste is polled by the watcher; update the history immediately too.
    add_text(&state, content)?;

    if let Ok(entries) = state.entries.lock() {
        emit_update(&app, &entries);
    }

    Ok(())
}
