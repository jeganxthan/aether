use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString, assets};

assets::icon_assets!(AdditionalIcons, [Wifi, EthernetPort]);

pub struct SettingsAssets;
pub const SIDEBAR_WIFI: &str = "assets/icons/sidebar/wifi.svg";
pub const SIDEBAR_ETHERNET: &str = "assets/icons/sidebar/ethernet.svg";
pub const WIFI_FULL: &str = "assets/icons/wifi/wifi-full.svg";
pub const WIFI_HALF: &str = "assets/icons/wifi/wifi-half.svg";
pub const WIFI_LOW: &str = "assets/icons/wifi/wifi-low.svg";
pub const WIFI_QUARTER: &str = "assets/icons/wifi/wifi-quater.svg";

pub fn svg_data(path: &str) -> &'static [u8] {
    match path {
        SIDEBAR_WIFI => include_bytes!("../assets/icons/sidebar/wifi.svg"),
        SIDEBAR_ETHERNET => include_bytes!("../assets/icons/sidebar/ethernet.svg"),
        WIFI_FULL => include_bytes!("../assets/icons/wifi/wifi-full.svg"),
        WIFI_HALF => include_bytes!("../assets/icons/wifi/wifi-half.svg"),
        WIFI_LOW => include_bytes!("../assets/icons/wifi/wifi-low.svg"),
        WIFI_QUARTER => include_bytes!("../assets/icons/wifi/wifi-quater.svg"),
        _ => panic!("unknown bundled SVG asset: {path}"),
    }
}

impl AssetSource for SettingsAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let local_icon = match path {
            SIDEBAR_WIFI | SIDEBAR_ETHERNET | WIFI_FULL | WIFI_HALF | WIFI_LOW | WIFI_QUARTER => {
                Some(svg_data(path))
            }
            _ => None,
        };
        if let Some(icon) = local_icon {
            return Ok(Some(Cow::Borrowed(icon)));
        }

        if let Some(asset) = AdditionalIcons.load(path)? {
            return Ok(Some(asset));
        }

        assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = assets::Assets.list(path)?;
        paths.extend(AdditionalIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_network_icons_and_default_icons() {
        let assets = SettingsAssets;

        for icon in [
            assets::IconName::Wifi,
            assets::IconName::EthernetPort,
            assets::IconName::Search,
        ] {
            assert!(
                assets.load(&icon.path()).unwrap().is_some(),
                "missing icon asset: {}",
                icon.path()
            );
        }

        for path in [
            SIDEBAR_WIFI,
            SIDEBAR_ETHERNET,
            WIFI_FULL,
            WIFI_HALF,
            WIFI_LOW,
            WIFI_QUARTER,
        ] {
            let bytes = svg_data(path);
            assert!(
                assets.load(path).unwrap().is_some(),
                "missing local icon asset: {path}"
            );
            let svg = std::str::from_utf8(bytes).unwrap();
            assert!(svg.starts_with("<svg "), "invalid SVG asset: {path}");
            assert!(svg.contains("<path"), "SVG has no path data: {path}");
        }
    }
}
