use gpui::{
    App, AppContext, Bounds, Size, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, point, px,
};

use gpui::layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions};

use crate::shell::panel::ShellPanel;

pub fn initialize(cx: &mut App) {
    cx.open_window(
        WindowOptions {
            titlebar: None,

            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: Size::new(px(0.0), px(32.0)),
            })),

            window_background: WindowBackgroundAppearance::Transparent,

            app_id: Some("aether-shell".into()),

            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "aether-panel".into(),

                layer: Layer::Top,

                anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,

                exclusive_zone: None,

                keyboard_interactivity: KeyboardInteractivity::None,

                ..Default::default()
            }),

            ..Default::default()
        },
        |_, cx| cx.new(ShellPanel::new),
    )
    .expect("Failed to create Aether layer-shell surface");

    cx.activate(true);
}
