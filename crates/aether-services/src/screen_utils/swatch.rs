use std::path::PathBuf;

use image::{Rgba, RgbaImage};

/*
 * A small square of the colour that was picked, so the popup can show the
 * colour itself and not only its name.
 */

// the size of the square, big enough to fill the avatar of the popup
const SIZE: u32 = 96;

// where the squares are kept, out of the way and out of the pictures folder
fn cache_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return Some(PathBuf::from(dir).join("aether"));
    }

    let home = std::env::var_os("HOME")?;

    Some(PathBuf::from(home).join(".cache/aether"))
}

// reads "#rrggbb" into its three channels
fn channels(hex: &str) -> Option<[u8; 3]> {
    let digits = hex.trim().trim_start_matches('#');

    if digits.len() != 6 {
        return None;
    }

    let mut channels = [0u8; 3];

    for (index, channel) in channels.iter_mut().enumerate() {
        *channel = u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16).ok()?;
    }

    Some(channels)
}

/*
 * Writes the square and hands back its path. The name holds the colour, so
 * every colour gets a file of its own and the shell never draws an old one.
 */
pub fn write(hex: &str) -> Option<PathBuf> {
    let [red, green, blue] = channels(hex)?;

    let dir = cache_dir()?;

    std::fs::create_dir_all(&dir).ok()?;

    let path = dir.join(format!("color-{hex}.png"));

    let square = RgbaImage::from_pixel(SIZE, SIZE, Rgba([red, green, blue, 255]));

    square.save(&path).ok()?;

    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_is_read_into_its_channels() {
        assert_eq!(channels("#ff8000"), Some([255, 128, 0]));
        assert_eq!(channels("ff8000"), Some([255, 128, 0]));
    }

    #[test]
    fn anything_that_is_not_a_colour_is_refused() {
        assert_eq!(channels("#fff"), None);
        assert_eq!(channels("#gggggg"), None);
        assert_eq!(channels(""), None);
    }
}
