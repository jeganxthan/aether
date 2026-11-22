
mod app;
mod shell;
mod theme;

use gpui::{App, QuitMode};

fn main() {
    gpui_platform::application()
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(|cx: &mut App| {
            app::initialize(cx);
        });
}
