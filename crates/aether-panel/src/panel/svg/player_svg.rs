use std::path::PathBuf;

use aether::Image;
use aether_services::player::PlayerBrand;

use super::recolor;
use crate::theme::theme;

pub fn player_brand_icon(brand: PlayerBrand) -> Image {
    let filename = match brand {
        PlayerBrand::Brave => "brave.svg",
        PlayerBrand::Spotify => "spotify.svg",
        PlayerBrand::Google => "google.svg",
        PlayerBrand::Other => "spotify.svg",
    };

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/music")
        .join(filename);

    recolor::ink_image(path, theme().icon)
}

pub fn control_icon(name: &str) -> Image {
    let filename = match name {
        "prev" => "prev.svg",
        "play" => "play-svgrepo-com.svg",
        "pause" => "pause-svgrepo-com.svg",
        "next" => "next-svgrepo-com.svg",
        _ => "play-svgrepo-com.svg",
    };

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons/music")
        .join(filename);

    recolor::ink_image(path, theme().text)
}
