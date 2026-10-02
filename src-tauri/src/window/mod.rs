use std::sync::Mutex;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, State};

/// Stable anchor for the widget: the horizontal centre and top edge of the
/// window, in logical pixels, captured once from the window's first position.
///
/// Stored rather than re-read on every resize on purpose. `set_position` and
/// `set_size` are queued onto the event loop, so querying the window's current
/// geometry mid-animation returns stale values. Feeding those stale values back
/// into a position calculation compounds the error on every frame and walks the
/// window across the screen.
#[derive(Default)]
pub struct Anchor(Mutex<Option<(f64, f64)>>);

#[tauri::command]
pub fn resize_window(
    app: AppHandle,
    anchor: State<'_, Anchor>,
    width: f64,
    height: f64,
) {
    let Some(window) = app.get_webview_window("aether-widget") else {
        return;
    };

    let (Ok(scale), Ok(position), Ok(size)) = (
        window.scale_factor(),
        window.outer_position(),
        window.inner_size(),
    ) else {
        return;
    };

    let mut guard = anchor.0.lock().unwrap_or_else(|e| e.into_inner());

    let (center_x, top_y) = match *guard {
        Some(anchor) => anchor,
        None => {
            let cur_x = f64::from(position.x) / scale;
            let cur_w = f64::from(size.width) / scale;
            let anchor = (cur_x + cur_w / 2.0, f64::from(position.y) / scale);
            *guard = Some(anchor);
            anchor
        }
    };

    drop(guard);

    // Derive the position purely from the stored anchor and the requested size,
    // so repeated calls with the same target are idempotent.
    let _ = window.set_position(LogicalPosition::new(center_x - width / 2.0, top_y));
    let _ = window.set_size(LogicalSize::new(width, height));
}