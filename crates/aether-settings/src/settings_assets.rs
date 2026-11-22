use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString, assets};

assets::icon_assets!(AdditionalIcons, [Wifi, EthernetPort]);

pub struct SettingsAssets;

impl AssetSource for SettingsAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
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
    }
}
