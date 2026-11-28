mod app;
mod components;
mod pages;
mod services;
mod settings_assets;
use gpui_kit::component::input::InputState;
use gpui_kit::*;

fn main() {
    let niri_blur = std::env::var_os("AETHER_NO_BLUR").is_none();

    application()
        .with_assets(settings_assets::SettingsAssets)
        .run(move |cx| {
            init(cx);
            aether_themes::init(cx);

            cx.open_window(
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
                    window_decorations: Some(WindowDecorations::Server),
                    ..Default::default()
                },
                |window, cx| {
                    let search_input =
                        cx.new(|cx| InputState::new(window, cx).placeholder("Search settings"));

                    let wifi_search_input =
                    cx.new(|cx| InputState::new(window, cx).placeholder("Search networks..."));
                    
                    let wifi_password_input =  
                                        cx.new(|cx| InputState::new(window, cx).placeholder("Search networks..."));

                    let view = cx.new(|cx| {
                        app::SettingsApp::new(
                            app::SettingsAppConfig {
                                search_input,
                                wifi_search_input,
                                wifi_password_input,
                                niri_blur,
                            },
                            cx,
                        )
                    });

                    cx.new(|cx| {
                        let root = base::Root::new(view, window, cx);
                        if niri_blur {
                            root.bg(rgba(0x00000000))
                        } else {
                            root
                        }
                    })
                },
            )
            .expect("Failed to open Aether Settings");
        });
}
