use std::path::PathBuf;

use aether::Image;

use super::recolor;
use crate::theme::theme;

const LEVELS: [u8; 10] = [0, 5, 10, 20, 35, 50, 65, 80, 90, 100];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryIcon {
    level: u8,
}

impl BatteryIcon {
    pub fn from_percentage(percentage: u8) -> Self {
        let percentage = percentage.min(100);

        let level = LEVELS
            .iter()
            .copied()
            .min_by_key(|level| level.abs_diff(percentage))
            .unwrap_or(0);

        Self { level }
    }

    pub fn level(self) -> u8 {
        self.level
    }

    pub fn asset_path(self, charging: bool, limited: bool) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/battery");

        let level = self.level;

        // Prefer the most specific variant, then fall back to available assets.
        let candidates: Vec<String> = match (charging, limited) {
            (true, true) => vec![
                format!("aether-applet-battery-level-{level}-limited-charging-symbolic.svg"),
                format!("aether-applet-battery-level-{level}-charging-symbolic.svg"),
                format!("aether-applet-battery-level-{level}-limited-symbolic.svg"),
                format!("aether-applet-battery-level-{level}-symbolic.svg"),
            ],
            (true, false) => vec![
                format!("aether-applet-battery-level-{level}-charging-symbolic.svg"),
                format!("aether-applet-battery-level-{level}-symbolic.svg"),
            ],
            (false, true) => vec![
                format!("aether-applet-battery-level-{level}-limited-symbolic.svg"),
                format!("aether-applet-battery-level-{level}-symbolic.svg"),
            ],
            (false, false) => vec![format!("aether-applet-battery-level-{level}-symbolic.svg")],
        };

        candidates
            .iter()
            .map(|name| root.join(name))
            .find(|path| path.exists())
            .unwrap_or_else(|| root.join("aether-applet-battery-level-100-symbolic.svg"))
    }

    /// Returns an image that Aether UI can load and display.
    pub fn image(self, charging: bool, limited: bool) -> Image {
        // the white outline takes the theme ink, while the charging green and the low red stay
        recolor::ink_image(self.asset_path(charging, limited), theme().icon)
    }

    /// Checks whether the selected image has finished loading.
    pub fn loaded(self, charging: bool, limited: bool) -> bool {
        Image::loaded(recolor::ink(
            &self.asset_path(charging, limited),
            theme().icon,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_nearest_level() {
        assert_eq!(BatteryIcon::from_percentage(0).level(), 0);
        assert_eq!(BatteryIcon::from_percentage(7).level(), 5);
        assert_eq!(BatteryIcon::from_percentage(67).level(), 65);
        assert_eq!(BatteryIcon::from_percentage(96).level(), 100);
        assert_eq!(BatteryIcon::from_percentage(150).level(), 100);
    }

    #[test]
    fn assets_resolve() {
        for percentage in [0, 5, 10, 20, 35, 50, 65, 80, 90, 100] {
            let icon = BatteryIcon::from_percentage(percentage);

            for charging in [false, true] {
                for limited in [false, true] {
                    let path = icon.asset_path(charging, limited);
                    assert!(path.exists(), "Missing battery icon: {}", path.display());
                }
            }
        }
    }

    #[test]
    fn svg_assets_load() {
        let icon = BatteryIcon::from_percentage(65);

        for _ in 0..100 {
            if icon.loaded(true, false) {
                return;
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        panic!("Battery SVG did not finish loading");
    }
}
