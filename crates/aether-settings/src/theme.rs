use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::{
    App,
    SharedString,
    component::{Theme, ThemeConfig, ThemeRegistry},
};

pub fn init(cx: &mut App) {
    if let Ok(config) = serde_json::from_str::<ThemeConfig>(include_str!("themes/aether-dark.json")) {
        Theme::update(cx, |theme| {
            theme.apply_config(&Rc::new(config));
        });
    }

    let theme_name = SharedString::from("Aether Dark");

    if let Err(error) = ThemeRegistry::watch_dir(
        PathBuf::from("./themes"),
        cx,
        move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx)
                .themes()
                .get(&theme_name)
                .cloned()
            {
                Theme::update(cx, |current| {
                    current.apply_config(&theme);
                });
            }
        },
    ) {
        eprintln!("Failed to load Aether theme: {error}");
    }
}