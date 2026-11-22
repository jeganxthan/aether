mod app;
mod components;
mod pages;
mod settings_assets;
mod theme;
use aether_services::ThemeService;
use gpui_kit::component::input::InputState;
use gpui_kit::*;

fn main() {
    let niri_blur = std::env::var_os("AETHER_NO_BLUR").is_none();

    application()
        .with_assets(settings_assets::SettingsAssets)
        .run(move |cx| {
            init(cx);
            ThemeService::initialize(cx);
            theme::init(cx);

            open_window(
                WindowOptions {
                    app_id: Some("aether-settings".to_string()),
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: point(px(100.0), px(100.0)),
                        size: Size::new(px(900.0), px(650.0)),
                    })),
                    window_background: if niri_blur {
                        WindowBackgroundAppearance::Blurred
                    } else {
                        WindowBackgroundAppearance::Opaque
                    },
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    let search_input =
                        cx.new(|cx| InputState::new(window, cx).placeholder("Search settings"));

                    cx.new(|cx| app::SettingsApp::new(search_input, niri_blur, cx))
                },
            )
            .expect("Failed to open Aether Settings");
        });
}
