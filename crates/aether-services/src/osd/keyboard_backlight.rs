use std::{fs, io, path::Path};

const LED_ROOT: &str = "/sys/class/leds";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardBacklightState {
    pub percent: u8,
    pub device: String,
}

pub struct KeyboardBacklightService;

impl KeyboardBacklightService {
    pub fn current() -> io::Result<KeyboardBacklightState> {
        for entry in fs::read_dir(LED_ROOT)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();

            if !is_keyboard_backlight(&name) {
                continue;
            }

            let path = entry.path();
            let Ok(percent) = read_percent(&path) else {
                continue;
            };

            return Ok(KeyboardBacklightState {
                percent,
                device: name,
            });
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No readable keyboard backlight found",
        ))
    }
}

fn is_keyboard_backlight(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains("kbd_backlight") || name.contains("keyboard")
}

fn read_percent(path: &Path) -> io::Result<u8> {
    let current: u64 = fs::read_to_string(path.join("brightness"))?
        .trim()
        .parse()
        .map_err(io::Error::other)?;
    let maximum: u64 = fs::read_to_string(path.join("max_brightness"))?
        .trim()
        .parse()
        .map_err(io::Error::other)?;

    if maximum == 0 {
        return Err(io::Error::other("Keyboard backlight maximum is zero"));
    }

    Ok(((current as f64 / maximum as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_keyboard_backlights() {
        assert!(is_keyboard_backlight("asus::kbd_backlight"));
        assert!(is_keyboard_backlight("Keyboard Backlight"));
        assert!(!is_keyboard_backlight("input3::capslock"));
        assert!(!is_keyboard_backlight("tpacpi::thinklight"));
    }

    #[test]
    fn reads_backlight_percentage() {
        let directory =
            std::env::temp_dir().join(format!("aether-kbd-backlight-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("brightness"), "2\n").unwrap();
        fs::write(directory.join("max_brightness"), "4\n").unwrap();

        assert_eq!(read_percent(&directory).unwrap(), 50);

        fs::remove_dir_all(directory).unwrap();
    }
}
