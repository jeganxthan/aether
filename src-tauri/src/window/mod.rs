use tauri::{AppHandle, LogicalSize, Manager, Runtime};

const WIDGET: &str = "aether-widget";

/// The island's width never changes, only its height.
///
/// This is not a limitation we settled for, it is forced by the compositor.
/// Under Wayland the position of a window is chosen by the compositor and the
/// client cannot move it afterwards — `gtk_window_move` is a no-op, which was
/// confirmed by instrumenting a real run: the position stayed at (0,0) no
/// matter what was requested. So the window grows from a fixed left edge. If the
/// width changed between the collapsed and expanded states, the window would
/// grow rightwards from that anchor and the island would visibly slide across
/// the screen as the pointer crossed the hover boundary.
///
/// Holding the width fixed and centring the island inside means the window never
/// moves horizontally, so the island stays anchored in one place. Only the
/// height animates, which Wayland does allow.
pub const ISLAND_WIDTH: f64 = 420.0;

/// Height of the island when collapsed.
pub const COLLAPSED_HEIGHT: f64 = 32.0;

/// Resizes the native window to the island's height.
///
/// The width is pinned to [`ISLAND_WIDTH`] and the island centres itself inside
/// it, so the widget keeps a stable horizontal position. The transparent margin
/// either side of the collapsed island is handled by the island asking the
/// compositor for click-through, not by shrinking the window.
#[tauri::command]
pub fn fit_window<R: Runtime>(app: AppHandle<R>, height: f64) {
    let Some(window) = app.get_webview_window(WIDGET) else {
        return;
    };

    let _ = window.set_size(LogicalSize::new(ISLAND_WIDTH, height));
}
