use std::path::PathBuf;

use aether::Image;

use super::recolor;
use crate::theme::theme;

// the icons the on screen display can show
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsdIcon {
    // audio
    VolumeLow,
    VolumeHalf,
    VolumeFull,
    Muted,

    // brightness
    BrightnessLow,
    BrightnessHigh,
}

impl OsdIcon {
    // the icon that fits a volume reading; muted always wins over the level
    pub fn volume(percent: u8, muted: bool) -> Self {
        if muted {
            return Self::Muted;
        }

        match percent {
            0..=33 => Self::VolumeLow,
            34..=66 => Self::VolumeHalf,
            _ => Self::VolumeFull,
        }
    }

    // the icon that fits a brightness reading
    pub fn brightness(percent: u8) -> Self {
        match percent {
            0..=49 => Self::BrightnessLow,
            _ => Self::BrightnessHigh,
        }
    }

    fn asset_name(self) -> &'static str {
        match self {
            Self::VolumeLow => "volume_low.svg",
            Self::VolumeHalf => "volume_half.svg",
            Self::VolumeFull => "volume_full.svg",
            Self::Muted => "mute.svg",
            Self::BrightnessLow => "sun-dim.svg",
            Self::BrightnessHigh => "sun.svg",
        }
    }

    fn asset_path(self) -> PathBuf {
        // the icons live in the workspace root, two levels above the panel crate
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/osd")
            .join(self.asset_name())
    }

    // ready to hand to a view; the icon is decoded and rasterized when the widget draws
    pub fn image(self) -> Image {
        recolor::ink_image(self.asset_path(), theme().icon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_resolve_from_the_panel_crate() {
        for icon in [
            OsdIcon::VolumeLow,
            OsdIcon::VolumeHalf,
            OsdIcon::VolumeFull,
            OsdIcon::Muted,
            OsdIcon::BrightnessLow,
            OsdIcon::BrightnessHigh,
        ] {
            let path = icon.asset_path();

            assert!(path.exists(), "missing asset: {}", path.display());
        }
    }

    #[test]
    fn the_volume_icon_follows_the_level_and_mute() {
        assert_eq!(OsdIcon::volume(0, false), OsdIcon::VolumeLow);
        assert_eq!(OsdIcon::volume(20, false), OsdIcon::VolumeLow);
        assert_eq!(OsdIcon::volume(50, false), OsdIcon::VolumeHalf);
        assert_eq!(OsdIcon::volume(80, false), OsdIcon::VolumeFull);
        assert_eq!(OsdIcon::volume(80, true), OsdIcon::Muted);
    }

    #[test]
    fn the_brightness_icon_splits_at_the_half() {
        assert_eq!(OsdIcon::brightness(0), OsdIcon::BrightnessLow);
        assert_eq!(OsdIcon::brightness(49), OsdIcon::BrightnessLow);
        assert_eq!(OsdIcon::brightness(50), OsdIcon::BrightnessHigh);
        assert_eq!(OsdIcon::brightness(100), OsdIcon::BrightnessHigh);
    }
}
