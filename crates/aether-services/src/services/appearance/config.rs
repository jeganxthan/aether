use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsConfig {
    pub appearance: AppearanceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub theme: String,
}
impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            appearance: AppearanceConfig {
                theme: "black".to_string(),
            },
        }
    }
}
impl SettingsConfig {
    pub fn config_path() -> io::Result<PathBuf> {
        let config_dir = match env::var_os("XDG_CONFIG_HOME") {
            Some(path) => PathBuf::from(path),
            None => {
                let home = env::var_os("HOME").ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "HOME environment variable not found",
                    )
                })?;
                PathBuf::from(home).join(".config")
            }
        };
        Ok(config_dir.join("aether").join("settings.toml"))
    }

    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;

        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let parent = path.parent().ok_or("Invalid config path")?;
        fs::create_dir_all(parent)?;

        let contents = toml::to_string_pretty(self)?;

        // Write to a temporary file first, then replace the old config.
        let temporary_path = path.with_extension("toml.tmp");

        fs::write(&temporary_path, contents)?;
        fs::rename(&temporary_path, path)?;

        Ok(())
    }
}
