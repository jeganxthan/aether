use std::fs;
use std::path::PathBuf;

use aether::Rectangle;

use crate::panel::svg::recolor::ink_image;
use crate::theme::theme;

const DISTRO_ICON_SIZE: f32 = 16.0;

fn distro_id() -> String {
    let Ok(contents) = fs::read_to_string("/etc/os-release") else {
        return "linux".to_string();
    };

    contents
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;

            if key == "ID" {
                Some(value.trim_matches('"').to_lowercase())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "linux".to_string())
}

fn distro_icon_path() -> PathBuf {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/distro");

    let icon = match distro_id().as_str() {
        "void" => "void.svg",
        "arch" => "arch.svg",
        "linuxmint" | "mint" => "mint.svg",
        "alpine" => "alphine.svg",
        _ => return assets.join("arch.svg"),
    };

    assets.join(icon)
}

pub fn distro_icon() -> Rectangle {
    let theme = theme();
    let path = distro_icon_path();
    Rectangle::new()
        .width(DISTRO_ICON_SIZE)
        .height(DISTRO_ICON_SIZE)
        .fill(ink_image(path, theme.icon))
}
