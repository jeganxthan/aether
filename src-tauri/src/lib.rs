mod clock;
mod commands;
mod error;
mod focus;
mod models;
mod services;
mod space;

use tauri::{Emitter, Manager};

use commands::clipboard::{
    clipboard_clear, clipboard_copy, clipboard_delete, clipboard_history, clipboard_pin,
    start_clipboard_watcher, ClipboardState,
};
use services::mpris::MediaService;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use services::notifications::{run_dbus, NotificationManager, NotificationRepository};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct IslandPointerEvent {
    pub kind: String,
    pub x: f64,
    pub y: f64,
}

/// Return keyboard focus to the window that was active before the island
/// (launcher / clipboard) took it. See `focus.rs` for how this is possible on
/// Wayland.
#[tauri::command]
fn focus_last_window(tracker: tauri::State<'_, focus::FocusTracker>) {
    tracker.restore();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }

                    let super_space = Shortcut::new(Some(Modifiers::SUPER), Code::Space);

                    if shortcut == &super_space {
                        if let Some(window) = app.get_webview_window("aether-widget") {
                            let _ = window.emit("launcher:toggle", ());
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            // Clock
            clock::get_time,
            // Battery
            commands::battery::battery_status,
            // Wi-Fi
            commands::wifi::wifi_status,
            commands::wifi::wifi_networks,
            commands::wifi::wifi_enable,
            commands::wifi::wifi_disable,
            commands::wifi::wifi_rescan,
            commands::wifi::wifi_connect,
            commands::wifi::wifi_disconnect,
            // Media / MPRIS
            commands::media::media_status,
            commands::media::media_play_pause,
            commands::media::media_next,
            commands::media::media_previous,
            commands::media::media_seek,
            commands::media::read_art,
            //Launcher
            commands::launcher::launcher_apps,
            commands::launcher::launcher_search,
            commands::launcher::launcher_icon,
            commands::launcher::launcher_launch,
            clipboard_history,
            clipboard_pin,
            clipboard_delete,
            clipboard_clear,
            clipboard_copy,
            // Notifications
            commands::notifications::notification_history,
            commands::notifications::notification_mark_read,
            commands::notifications::notification_dismiss,
            commands::notifications::notification_clear_history,
            commands::window::set_input_shape,
            commands::window::debug_log,
            commands::window::set_desktop_margin,
            focus_last_window,
            commands::system_controls::volume_get,
            commands::system_controls::volume_increase,
            commands::system_controls::volume_decrease,
            commands::system_controls::volume_toggle_mute,
            commands::system_controls::brightness_get,
            commands::system_controls::brightness_increase,
            commands::system_controls::brightness_decrease,
        ])
        .setup(|app| {
            if let Err(error) = app
                .global_shortcut()
                .register(Shortcut::new(Some(Modifiers::SUPER), Code::Space))
            {
                eprintln!("[Aether] Failed to register Super+Space: {error}");
            }

            // Start clock
            clock::start(app.handle().clone());

            // Track which toplevel held focus so the launcher / clipboard can
            // hand it back when they close (see `focus.rs`).
            if focus::FocusTracker::start()
                .map(|t| app.manage(t))
                .is_none()
            {
                eprintln!("[Aether] Focus tracker unavailable; focus restore disabled");
            }

            // Reserve the top strip with a transparent layer-shell surface so
            // maximized / tiled windows stay below the island. The exclusive
            // zone is static — it is never toggled (see `space.rs`).
            if space::SpaceReserver::start()
                .map(|s| app.manage(s))
                .is_none()
            {
                eprintln!("[Aether] Layer-shell unavailable; top strip not reserved");
            }

            // ---------------------------------------------------------
            // NOTIFICATION SERVICE
            // ---------------------------------------------------------

            let database_path = app.path().app_data_dir()?.join("notifications.db");

            let repository =
                NotificationRepository::new(&database_path).map_err(std::io::Error::other)?;

            let notification_manager = NotificationManager::new(app.handle().clone(), repository)
                .map_err(std::io::Error::other)?;

            app.manage(notification_manager.clone());

            tauri::async_runtime::spawn(async move {
                if let Err(error) = run_dbus(notification_manager).await {
                    eprintln!("[Aether Notifications] Service failed: {error}");
                }
            });

            // ---------------------------------------------------------
            // CLIPBOARD HISTORY
            // ---------------------------------------------------------

            // The history lives under the XDG data dir and the watcher polls
            // the system clipboard (wl-clipboard) for new entries. State must
            // be managed before the watcher touches app.state().
            let clipboard_path = dirs::data_dir()
                .map(|dir| dir.join("aether").join("clipboard.json"))
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp/aether-clipboard.json"));

            app.manage(ClipboardState::new(clipboard_path));
            start_clipboard_watcher(app.handle().clone());

            // ---------------------------------------------------------
            // LAUNCHER + CLIPBOARD TRIGGERS (Wayland / labwc)
            // ---------------------------------------------------------

            // A Wayland compositor never delivers key grabs to clients, so
            // tauri-plugin-global-shortcut cannot drive hotkeys under labwc.
            // Instead the compositor's rc.xml runs `pkill -<signal> -x
            // aether-tauri`, and this thread turns each signal into the event
            // the frontend listens for:
            //   SIGUSR2  -> launcher:toggle  (Super+Space)
            //   SIGWINCH -> clipboard:toggle (Super+C)
            {
                // NOTE: WebKitGTK (JavaScriptCore) uses SIGUSR1 internally for
                // thread suspension, so installing a handler for it crashes the
                // web view. SIGUSR2 and SIGWINCH are free.
                use signal_hook::consts::{SIGUSR2, SIGWINCH};
                use signal_hook::iterator::Signals;

                let app_handle = app.handle().clone();

                std::thread::spawn(move || {
                    // Volume / brightness keys arrive as realtime signals.
                    // SIGRTMIN is not a compile-time constant on glibc, so it
                    // is resolved at runtime; SIGRTMIN()+0..4 are free for the
                    // process.
                    let vol_up = libc::SIGRTMIN();
                    let vol_down = vol_up + 1;
                    let vol_mute = vol_up + 2;
                    let bright_up = vol_up + 3;
                    let bright_down = vol_up + 4;

                    let mut signals = match Signals::new([
                        SIGUSR2,
                        SIGWINCH,
                        vol_up,
                        vol_down,
                        vol_mute,
                        bright_up,
                        bright_down,
                    ]) {
                        Ok(signals) => signals,
                        Err(error) => {
                            eprintln!("[Aether] Failed to install signal handler: {error}");
                            return;
                        }
                    };

                    let focus = |app_handle: &tauri::AppHandle| {
                        if let Some(window) = app_handle.get_webview_window("aether-widget") {
                            let _ = window.set_focus();
                            std::thread::spawn({
                                let window = window.clone();
                                move || {
                                    std::thread::sleep(std::time::Duration::from_millis(120));
                                    let focused = window.is_focused().unwrap_or(false);
                                    crate::commands::window::trace(&format!(
                                        "after set_focus focused={focused}"
                                    ));
                                }
                            });
                        }
                    };

                    for signal in signals.forever() {
                        match signal {
                            SIGUSR2 => {
                                let _ = app_handle.emit("launcher:toggle", ());
                                focus(&app_handle);
                            }
                            SIGWINCH => {
                                let _ = app_handle.emit("clipboard:toggle", ());
                                focus(&app_handle);
                            }
                            // Volume / brightness: show the island OSD, but do
                            // NOT steal focus — it is a passive overlay.
                            signal if signal == vol_up => {
                                let _ = app_handle.emit(
                                    "system:control",
                                    serde_json::json!({ "kind": "volume", "action": "up" }),
                                );
                            }
                            signal if signal == vol_down => {
                                let _ = app_handle.emit(
                                    "system:control",
                                    serde_json::json!({ "kind": "volume", "action": "down" }),
                                );
                            }
                            signal if signal == vol_mute => {
                                let _ = app_handle.emit(
                                    "system:control",
                                    serde_json::json!({ "kind": "volume", "action": "mute" }),
                                );
                            }
                            signal if signal == bright_up => {
                                let _ = app_handle.emit(
                                    "system:control",
                                    serde_json::json!({ "kind": "brightness", "action": "up" }),
                                );
                            }
                            signal if signal == bright_down => {
                                let _ = app_handle.emit(
                                    "system:control",
                                    serde_json::json!({ "kind": "brightness", "action": "down" }),
                                );
                            }
                            _ => {}
                        }
                    }
                });
            }

            // ---------------------------------------------------------
            // MPRIS MEDIA SERVICE
            // ---------------------------------------------------------

            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match MediaService::new(app_handle.clone()).await {
                    Ok(media) => {
                        // Make MediaService available to Tauri commands.
                        app_handle.manage(media.clone());

                        // Start listening to MPRIS / D-Bus.
                        media.start().await;
                    }

                    Err(err) => {
                        eprintln!("[Aether Media] Failed to initialize MPRIS: {err}");
                    }
                }
            });

            // ---------------------------------------------------------
            // WINDOW
            // ---------------------------------------------------------

            // Under Wayland / labwc, the window is positioned at the top-center
            // by compositor window rules, but transparent webview pixels would
            // otherwise intercept desktop pointer events. Setting an initial
            // input shape restricts click interception to the collapsed pill.
            if let Some(window) = app.get_webview_window("aether-widget") {
                #[cfg(target_os = "linux")]
                if let Ok(gtk_win) = window.gtk_window() {
                    use gtk::cairo::{RectangleInt, Region};
                    use gtk::prelude::*;
                    let rect = RectangleInt::new(222, 0, 116, 28);
                    let region = Region::create_rectangle(&rect);
                    gtk_win.input_shape_combine_region(Some(&region));

                    // GDK receives pointer crossing events straight from the
                    // compositor. WebKitGTK sometimes withholds the DOM leave
                    // when the input region changes, so hover is driven from
                    // GDK instead and forwarded to the frontend, with the
                    // pointer position so the island can tell real hovers from
                    // transient region-change artifacts.
                    gtk_win.add_events(
                        gtk::gdk::EventMask::ENTER_NOTIFY_MASK
                            | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK
                            | gtk::gdk::EventMask::POINTER_MOTION_MASK,
                    );
                    // Any crossing that leaves or enters the toplevel (moving
                    // between child windows, Inferior, is not a real hover).
                    let is_toplevel = |d: gtk::gdk::NotifyType| d != gtk::gdk::NotifyType::Inferior;
                    let last_motion: std::rc::Rc<std::cell::Cell<f64>> =
                        std::rc::Rc::new(std::cell::Cell::new(0.0));

                    let hover_handle = app.handle().clone();
                    gtk_win.connect_enter_notify_event(move |_, e| {
                        if is_toplevel(e.detail()) {
                            let _ = hover_handle.emit(
                                "island:pointer",
                                IslandPointerEvent {
                                    kind: "enter".to_string(),
                                    x: e.position().0,
                                    y: e.position().1,
                                },
                            );
                        }
                        gtk::glib::Propagation::Proceed
                    });
                    let leave_handle = app.handle().clone();
                    gtk_win.connect_leave_notify_event(move |_, e| {
                        if is_toplevel(e.detail()) {
                            let _ = leave_handle.emit(
                                "island:pointer",
                                IslandPointerEvent {
                                    kind: "leave".to_string(),
                                    x: e.position().0,
                                    y: e.position().1,
                                },
                            );
                        }
                        gtk::glib::Propagation::Proceed
                    });
                    let motion_handle = app.handle().clone();
                    let motion_throttle = last_motion.clone();
                    gtk_win.connect_motion_notify_event(move |_, e| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as f64)
                            .unwrap_or(0.0);
                        if now - motion_throttle.get() < 30.0 {
                            return gtk::glib::Propagation::Proceed;
                        }
                        motion_throttle.set(now);
                        let _ = motion_handle.emit(
                            "island:pointer",
                            IslandPointerEvent {
                                kind: "motion".to_string(),
                                x: e.position().0,
                                y: e.position().1,
                            },
                        );
                        gtk::glib::Propagation::Proceed
                    });
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
