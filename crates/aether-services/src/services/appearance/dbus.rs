use std::sync::Arc;

use crate::theme::ThemeKind;
use zbus::{interface, object_server::SignalEmitter};

use super::manager::ThemeManager;

pub struct AppearanceService {
    manager: Arc<ThemeManager>,
}

impl AppearanceService {
    pub fn new(manager: Arc<ThemeManager>) -> Self {
        Self { manager }
    }

    fn parse_theme(value: &str) -> zbus::fdo::Result<ThemeKind> {
        ThemeKind::from_name(value)
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs("Theme must be Black or White".into()))
    }
}

#[interface(name = "org.aether.Appearance")]
impl AppearanceService {
    async fn get_theme(&self) -> String {
        self.manager.current().await.name().to_string()
    }

    async fn set_theme(
        &self,
        theme: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let theme = Self::parse_theme(&theme)?;

        let changed = self
            .manager
            .set_theme(theme)
            .await
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        if changed {
            Self::theme_changed(&emitter, theme.name())
                .await
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
        }

        Ok(())
    }

    #[zbus(signal)]
    async fn theme_changed(emitter: &SignalEmitter<'_>, theme: &str) -> zbus::Result<()>;
}
