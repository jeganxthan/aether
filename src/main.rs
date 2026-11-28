use gpui::{
    App, Context, FontWeight, Window, WindowKind, WindowOptions, div, layer_shell::*, prelude::*,
    px, rgb,
};
use gpui_platform::application;

struct AetherShell;

impl Render for AetherShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .px_4()
            .rounded_xl()
            .bg(rgb(0x20232b))
            .text_color(rgb(0xffffff))
            .font_weight(FontWeight::MEDIUM)
            .child("Aether shell running on Aether")
    }
}

fn main() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                titlebar: None,
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "aether".to_string(),
                    layer: Layer::Top,
                    anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
                    exclusive_zone: Some(px(52.)),
                    margin: Some((px(8.), px(12.), px(0.), px(12.))),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    ..Default::default()
                }),
                window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds {
                    origin: gpui::point(px(0.), px(0.)),
                    size: gpui::Size::new(px(800.), px(40.)),
                })),
                ..Default::default()
            },
            |_, cx| cx.new(|_| AetherShell),
        )
        .expect("failed to open Aether layer-shell window");
    });
}


// cd /home/jegan/Desktop/aether &&
// sudo install -Dm755 crates/compositor/target/release/aether /usr/local/bin/aether &&
// cmp -s crates/compositor/target/release/aether /usr/local/bin/aether &&
// echo "Latest compositor installed and verified"