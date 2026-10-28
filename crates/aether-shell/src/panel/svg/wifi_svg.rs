use std::path::PathBuf;

use aether::Image;

use super::recolor;
use crate::theme::theme;

#[derive(Debug, Clone, Copy)]
pub enum WifiSignalIcon {
    None,
    Weak,
    Ok,
    Good,
    Excellent,
}

impl WifiSignalIcon {
    pub fn from_strength(strength: u8) -> Self {
        match strength {
            80..=100 => Self::Excellent,
            60..=79 => Self::Good,
            40..=59 => Self::Ok,
            1..=39 => Self::Weak,
            _ => Self::None,
        }
    }

    pub fn asset_name(self, secured: bool) -> &'static str {
        match (self, secured) {
            (Self::Excellent, true) => "network-wireless-secure-signal-excellent-symbolic.svg",
            (Self::Good, true) => "network-wireless-secure-signal-good-symbolic.svg",
            (Self::Ok, true) => "network-wireless-secure-signal-ok-symbolic.svg",
            (Self::Weak, true) => "network-wireless-secure-signal-weak-symbolic.svg",
            (Self::None, true) => "network-wireless-secure-signal-none-symbolic.svg",

            (Self::Excellent, false) => "network-wireless-signal-excellent-symbolic.svg",
            (Self::Good, false) => "network-wireless-signal-good-symbolic.svg",
            (Self::Ok, false) => "network-wireless-signal-ok-symbolic.svg",
            (Self::Weak, false) => "network-wireless-signal-weak-symbolic.svg",
            (Self::None, false) => "network-wireless-signal-none-symbolic.svg",
        }
    }

    pub fn asset_path(self, secured: bool) -> PathBuf {
        // the icons live in the workspace root, two levels above the panel crate
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/wifi")
            .join(self.asset_name(secured))
    }

    // ready to hand to a view; the icon is decoded and rasterized when the widget draws
    pub fn image(self, secured: bool) -> Image {
        recolor::ink_image(self.asset_path(secured), theme().icon)
    }

    // the same, drawn in a chosen ink, for a row that carries its own color
    pub fn image_ink(self, secured: bool, ink: aether::Color) -> Image {
        recolor::ink_image(self.asset_path(secured), ink)
    }

    // whether the asset is decoded yet, for a view that shows a placeholder until then
    pub fn loaded(self, secured: bool) -> bool {
        Image::loaded(recolor::ink(&self.asset_path(secured), theme().icon))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_resolve_from_the_panel_crate() {
        // every icon the enum can ask for has to exist on disk with the name built here
        for secured in [true, false] {
            for icon in [
                WifiSignalIcon::None,
                WifiSignalIcon::Weak,
                WifiSignalIcon::Ok,
                WifiSignalIcon::Good,
                WifiSignalIcon::Excellent,
            ] {
                let path = icon.asset_path(secured);

                assert!(path.exists(), "missing asset: {}", path.display());
            }
        }
    }

    #[test]
    fn svg_assets_rasterize() {
        // the public loader routes a .svg through the rasterizer; the first call starts a
        // background decode and reports none, so the icon shows up a moment later
        let icon = WifiSignalIcon::Good;

        for _ in 0..100 {
            if icon.loaded(true) {
                return;
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        panic!("the svg never finished rasterizing");
    }
}
