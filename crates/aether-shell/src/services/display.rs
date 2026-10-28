use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use crate::services::runtime::runtime;

static WATCHER_STARTED: OnceLock<()> = OnceLock::new();

fn primary_display_path() -> Option<PathBuf> {
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(config_dir.join("aether-shell/primary-display"))
}

pub fn primary_display() -> Option<String> {
    let path = primary_display_path()?;
    match std::fs::read_to_string(path) {
        Ok(name) => {
            let name = name.trim();
            (!name.is_empty()).then(|| name.to_owned())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            eprintln!("aether-panel: could not read primary display setting: {error}");
            None
        }
    }
}

pub fn is_primary(
    name: &str,
    x: i32,
    y: i32,
    outputs: &[String],
    origin_output: Option<&str>,
) -> bool {
    let configured = primary_display();
    let selected = select_output(configured.as_deref(), outputs, origin_output);

    selected
        .as_deref()
        .map_or(x == 0 && y == 0, |selected| selected == name)
}

fn select_output(
    configured: Option<&str>,
    outputs: &[String],
    origin_output: Option<&str>,
) -> Option<String> {
    configured
        .filter(|primary| outputs.iter().any(|output| output == primary))
        .map(str::to_owned)
        .or_else(|| origin_output.map(str::to_owned))
        .or_else(|| outputs.first().cloned())
}

pub fn watch_primary_display() {
    WATCHER_STARTED.get_or_init(|| {
        runtime().spawn(async {
            let mut last = primary_display();
            let mut interval = tokio::time::interval(Duration::from_millis(500));

            loop {
                interval.tick().await;
                let current = primary_display();
                if current != last {
                    last = current;
                    aether::refresh();
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_primary_is_used_while_connected() {
        let outputs = vec!["DP-1".to_owned(), "HDMI-A-1".to_owned()];

        assert_eq!(
            select_output(Some("HDMI-A-1"), &outputs, Some("DP-1")),
            Some("HDMI-A-1".to_owned())
        );
    }

    #[test]
    fn disconnected_primary_falls_back_to_the_origin_output() {
        let outputs = vec!["DP-1".to_owned(), "HDMI-A-1".to_owned()];

        assert_eq!(
            select_output(Some("eDP-1"), &outputs, Some("DP-1")),
            Some("DP-1".to_owned())
        );
    }

    #[test]
    fn missing_primary_and_origin_fall_back_to_a_connected_output() {
        let outputs = vec!["DP-1".to_owned(), "HDMI-A-1".to_owned()];

        assert_eq!(select_output(None, &outputs, None), Some("DP-1".to_owned()));
    }
}
