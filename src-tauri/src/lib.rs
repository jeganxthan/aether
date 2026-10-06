mod clock;
mod window;
mod commands;
mod services;
mod models;
mod error;

use tauri::Manager;

use services::mpris::MediaService;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Clock
            clock::get_time,

            // Window
            window::fit_window,

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
        ])

        .setup(|app| {
            // Start clock
            clock::start(app.handle().clone());

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
                        eprintln!(
                            "[Aether Media] Failed to initialize MPRIS: {err}"
                        );
                    }
                }
            });

            // ---------------------------------------------------------
            // WINDOW
            // ---------------------------------------------------------

            // Only the height is set here. The width is pinned for the
            // lifetime of the process because Wayland will not let the
            // window be moved.
            if let Some(window) =
                app.get_webview_window("aether-widget")
            {
                let _ = window.set_size(
                    tauri::LogicalSize::new(
                        window::ISLAND_WIDTH,
                        window::COLLAPSED_HEIGHT,
                    ),
                );
            }

            Ok(())
        })

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}