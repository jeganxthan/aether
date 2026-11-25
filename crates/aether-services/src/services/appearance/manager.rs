use std::{path::PathBuf, sync::Arc};

use aether_themes::ThemeKind;
use tokio::sync::RwLock;

use super::config::SettingsConfig;

type ServiceResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct ThemeManager {
    current: RwLock<ThemeKind>,
    config_path: PathBuf,
}

impl ThemeManager {
    pub fn load() -> ServiceResult<Arc<Self>> {
        let config_path = SettingsConfig::config_path()?;
        let config = SettingsConfig::load(&config_path)?;

        let initial_theme = match config.appearance.theme.to_lowercase().as_str() {
            "black" => ThemeKind::Black,
            "white" => ThemeKind::White,
            other => {
                return Err(format!("Invalid saved theme: {other}").into());
            }
        };

        // Create the default config on the first launch.
        if !config_path.exists() {
            config.save(&config_path)?;
        }

        Ok(Arc::new(Self {
            current: RwLock::new(initial_theme),
            config_path,
        }))
    }

    pub async fn current(&self) -> ThemeKind {
        *self.current.read().await
    }

    pub async fn set_theme(&self, theme: ThemeKind) -> ServiceResult<bool> {
        let mut current = self.current.write().await;

        if *current == theme {
            return Ok(false);
        }

        let mut config = SettingsConfig::load(&self.config_path)?;

        config.appearance.theme = match theme {
            ThemeKind::Black => "black",
            ThemeKind::White => "white",
        }
        .to_string();

        // Persist first. Don't change the in-memory theme if saving fails.
        config.save(&self.config_path)?;

        *current = theme;

        Ok(true)
    }
}
