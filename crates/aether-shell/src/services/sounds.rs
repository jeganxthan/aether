use std::path::Path;
use std::process::Command;

use aether_services::notifications::Notification;

pub fn play_notification(notification: &Notification) {
    let sound = if is_low_battery(notification) {
        Sound::File("low-battery.mp3")
    } else if is_high_battery(notification) {
        Sound::File("batteryfull.mp3")
    } else {
        Sound::Bundled("notification.wav")
    };

    match sound {
        Sound::File(name) => play_asset(name),
        Sound::Bundled(name) => play_asset(name),
    }
}

enum Sound {
    File(&'static str),
    Bundled(&'static str),
}

fn is_low_battery(notification: &Notification) -> bool {
    notification
        .summary
        .to_ascii_lowercase()
        .contains("low battery")
}

fn is_high_battery(notification: &Notification) -> bool {
    notification
        .summary
        .to_ascii_lowercase()
        .contains("battery fully charged")
}

fn play_asset(name: &str) {
    let path = if name == "notification.wav" {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/sounds")
            .join(name)
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/battery sound")
            .join(name)
    };

    if !path.is_file() {
        eprintln!(
            "aether-panel: battery notification sound is missing: {}",
            path.display()
        );
        return;
    }

    let result = Command::new("pw-play")
        .arg(&path)
        .spawn()
        .or_else(|_| Command::new("paplay").arg(&path).spawn());

    if let Err(error) = result {
        eprintln!(
            "aether-panel: could not play battery notification sound {}: {error}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(summary: &str) -> Notification {
        Notification::new(
            1,
            "Aether".into(),
            String::new(),
            String::new(),
            summary.into(),
            String::new(),
            Vec::new(),
            5000,
        )
    }

    #[test]
    fn battery_summaries_select_distinct_alerts() {
        assert!(is_low_battery(&notification("Low battery")));
        assert!(is_high_battery(&notification("Battery fully charged")));
        assert!(!is_low_battery(&notification("Battery charging")));
    }
}
